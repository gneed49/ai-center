// @vitest-environment jsdom
/// <reference types="node" />
import { webcrypto } from "node:crypto";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { providersApi } from "@/api/providers";
import { setRequestIdentity } from "@/api/request-context";
import type { Provider, ProviderConnection } from "@/api/provider-types";
import { ConnectionForm } from "./connection-form";

vi.mock("@/api/providers", () => ({
  providersApi: { create: vi.fn(), update: vi.fn() },
}));
const provider: Provider = {
  id: "openai",
  name: "OpenAI",
  auth_method: "api_key",
  available: true,
};
const connection: ProviderConnection = {
  id: "fixture-id",
  provider: "openai",
  name: "Personnel",
  model: "model-a",
  key_hint: "••••1234",
  created_at: "2026-09-07",
  updated_at: "2026-09-07",
};

beforeEach(() => {
  vi.resetAllMocks();
  vi.stubGlobal("crypto", webcrypto);
  setRequestIdentity("actor-form", "workspace-form", "fixture-token");
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("connection form unit behavior", () => {
  it("submits a key once and clears the password after success", async () => {
    vi.mocked(providersApi.create).mockResolvedValue(connection);
    const saved = vi.fn();
    render(
      <ConnectionForm provider={provider} storageAvailable onSaved={saved} />,
    );
    fireEvent.change(screen.getByLabelText("Nom de la connexion"), {
      target: { value: "Personnel" },
    });
    fireEvent.change(screen.getByLabelText("Modèle"), {
      target: { value: "model-a" },
    });
    const input = screen.getByLabelText<HTMLInputElement>("Clé API");
    expect(input.type).toBe("password");
    fireEvent.change(input, { target: { value: "fixture-only-key" } });
    fireEvent.submit(screen.getByRole("form"));
    await waitFor(() => expect(saved).toHaveBeenCalledWith(connection));
    expect(vi.mocked(providersApi.create).mock.calls[0][0]).toMatchObject({
      provider: "openai",
      api_key: "fixture-only-key",
    });
    expect(input.value).toBe("");
    expect(localStorage.length).toBe(0);
    expect(sessionStorage.length).toBe(0);
  });
  it("keeps the saved key when only changing name/model", async () => {
    vi.mocked(providersApi.update).mockResolvedValue(connection);
    render(
      <ConnectionForm
        provider={provider}
        connection={connection}
        storageAvailable
        onSaved={vi.fn()}
      />,
    );
    fireEvent.change(screen.getByLabelText("Modèle"), {
      target: { value: "model-b" },
    });
    fireEvent.submit(screen.getByRole("form"));
    await waitFor(() => expect(providersApi.update).toHaveBeenCalled());
    expect(vi.mocked(providersApi.update).mock.calls[0][1]).toEqual({
      name: "Personnel",
      model: "model-b",
    });
  });
  it("retries the same failed creation with the same ID and idempotency key", async () => {
    vi.mocked(providersApi.create)
      .mockRejectedValueOnce(new Error("fixture-private-key"))
      .mockResolvedValueOnce(connection);
    render(
      <ConnectionForm provider={provider} storageAvailable onSaved={vi.fn()} />,
    );
    fireEvent.change(screen.getByLabelText("Nom de la connexion"), {
      target: { value: "Personnel" },
    });
    fireEvent.change(screen.getByLabelText("Modèle"), {
      target: { value: "model-a" },
    });
    fireEvent.change(screen.getByLabelText("Clé API"), {
      target: { value: "fixture-private-key" },
    });
    fireEvent.submit(screen.getByRole("form"));
    await screen.findByRole("alert");
    expect(screen.getByRole("alert").textContent).not.toContain(
      "fixture-private-key",
    );
    fireEvent.submit(screen.getByRole("form"));
    await waitFor(() => expect(providersApi.create).toHaveBeenCalledTimes(2));
    expect(vi.mocked(providersApi.create).mock.calls[1]).toEqual(
      vi.mocked(providersApi.create).mock.calls[0],
    );
  });
  it("cannot save a key when secure storage is unavailable", () => {
    render(
      <ConnectionForm
        provider={provider}
        storageAvailable={false}
        onSaved={vi.fn()}
      />,
    );
    expect(screen.getByLabelText<HTMLInputElement>("Clé API").disabled).toBe(
      true,
    );
    fireEvent.submit(screen.getByRole("form"));
    expect(providersApi.create).not.toHaveBeenCalled();
  });
  it("does not display a key field for subscriptions", () => {
    render(
      <ConnectionForm
        provider={{ ...provider, auth_method: "subscription" }}
        storageAvailable={false}
        onSaved={vi.fn()}
      />,
    );
    expect(screen.queryByLabelText("Clé API")).toBeNull();
    expect(
      screen.getByRole<HTMLButtonElement>("button", {
        name: "Ajouter la connexion",
      }).disabled,
    ).toBe(false);
  });
  it("drops a pending result and clears the key when its form scope unmounts", async () => {
    let finish!: (value: ProviderConnection) => void;
    vi.mocked(providersApi.update).mockImplementation(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    const saved = vi.fn();
    const { rerender } = render(
      <ConnectionForm
        key="scope-a"
        provider={provider}
        connection={connection}
        storageAvailable
        onSaved={saved}
      />,
    );
    fireEvent.change(screen.getByLabelText("Nouvelle clé API (facultatif)"), {
      target: { value: "fixture-only-key" },
    });
    fireEvent.submit(screen.getByRole("form"));
    await waitFor(() => expect(providersApi.update).toHaveBeenCalled());
    rerender(
      <ConnectionForm
        key="scope-b"
        provider={provider}
        connection={connection}
        storageAvailable
        onSaved={saved}
      />,
    );
    finish(connection);
    await Promise.resolve();
    expect(
      screen.getByLabelText<HTMLInputElement>("Nouvelle clé API (facultatif)")
        .value,
    ).toBe("");
    expect(saved).not.toHaveBeenCalled();
  });
});
