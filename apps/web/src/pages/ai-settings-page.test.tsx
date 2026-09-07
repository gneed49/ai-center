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
  within,
} from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { providersApi } from "@/api/providers";
import type { ProviderSettings } from "@/api/provider-types";
import { setRequestIdentity } from "@/api/request-context";
import { AiSettingsPage } from "./ai-settings-page";

const auth = vi.hoisted(() => ({
  workspaceId: "workspace-a",
  session: { user: { id: "actor-a" } },
}));
vi.mock("@/auth/auth-context", () => ({ useAuth: () => auth }));
vi.mock("@/api/providers", () => ({
  providersApi: {
    settings: vi.fn(),
    create: vi.fn(),
    remove: vi.fn(),
    select: vi.fn(),
  },
}));
const settings: ProviderSettings = {
  providers: [
    { id: "openai", name: "OpenAI", auth_method: "api_key", available: true },
    { id: "kimi", name: "Kimi", auth_method: "api_key", available: true },
    {
      id: "codex_subscription",
      name: "ChatGPT via Codex",
      auth_method: "subscription",
      available: false,
      unavailable_reason: "Client compatible requis.",
    },
  ],
  connections: [],
  selection: { mode: "deterministic", connection_id: null },
  storage: { available: true },
};
const masked = {
  id: "profile-a",
  provider: "openai",
  name: "Personnel",
  model: "model-a",
  key_hint: "••••1234",
  created_at: "2026-09-07",
  updated_at: "2026-09-07",
};
let client: QueryClient;
beforeEach(() => {
  vi.resetAllMocks();
  vi.stubGlobal("crypto", webcrypto);
  auth.workspaceId = "workspace-a";
  setRequestIdentity("actor-a", "workspace-a", "fixture-token");
  client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  vi.mocked(providersApi.settings).mockResolvedValue(settings);
});
afterEach(() => {
  cleanup();
  client.clear();
  vi.unstubAllGlobals();
});
const page = () => (
  <QueryClientProvider client={client}>
    <AiSettingsPage />
  </QueryClientProvider>
);

describe("settings page with an in-memory API", () => {
  it("stores only masked metadata in its query cache and never uses a mutation cache for the key", async () => {
    vi.mocked(providersApi.create).mockResolvedValue(masked);
    render(page());
    const form = await screen.findByRole("form", {
      name: "Nouvelle connexion IA",
    });
    fireEvent.change(within(form).getByLabelText("Nom de la connexion"), {
      target: { value: "Personnel" },
    });
    fireEvent.change(within(form).getByLabelText("Modèle"), {
      target: { value: "model-a" },
    });
    fireEvent.change(within(form).getByLabelText("Clé API"), {
      target: { value: "fixture-private-secret" },
    });
    fireEvent.submit(form);
    await screen.findByRole("heading", { name: "Personnel" });
    const queryData = client
      .getQueryCache()
      .getAll()
      .map((query) => query.state.data);
    expect(JSON.stringify(queryData)).toContain("••••1234");
    expect(JSON.stringify(queryData)).not.toContain("fixture-private-secret");
    expect(client.getMutationCache().getAll()).toHaveLength(0);
    expect(screen.getByLabelText<HTMLInputElement>("Clé API").value).toBe("");
  });
  it("resets a partially entered secret when the workspace changes", async () => {
    const rendered = render(page());
    const input = await screen.findByLabelText<HTMLInputElement>("Clé API");
    fireEvent.change(input, { target: { value: "fixture-private-secret" } });
    auth.workspaceId = "workspace-b";
    setRequestIdentity("actor-a", "workspace-b", "fixture-token");
    rendered.rerender(page());
    const next = await screen.findByLabelText<HTMLInputElement>("Clé API");
    expect(next).not.toBe(input);
    expect(next.value).toBe("");
    expect(providersApi.create).not.toHaveBeenCalled();
  });
  it("displays distinct profiles for the same provider and explains unavailable subscriptions", async () => {
    vi.mocked(providersApi.settings).mockResolvedValue({
      ...settings,
      connections: [masked, { ...masked, id: "profile-b", name: "Travail" }],
    });
    render(page());
    await screen.findByRole("heading", { name: "Personnel" });
    expect(screen.getByRole("heading", { name: "Travail" })).toBeDefined();
    expect(
      screen.getByText("Client compatible requis.", { exact: false }),
    ).toBeDefined();
    expect(
      screen.queryByRole("button", { name: "Connecter mon abonnement" }),
    ).toBeNull();
  });
  it("removes the active profile and reflects the server's deterministic fallback", async () => {
    vi.mocked(providersApi.settings)
      .mockResolvedValueOnce({
        ...settings,
        connections: [masked],
        selection: { mode: "connection", connection_id: masked.id },
      })
      .mockResolvedValue(settings);
    vi.mocked(providersApi.remove).mockResolvedValue({ deleted: true });
    render(page());
    fireEvent.click(await screen.findByRole("button", { name: "Supprimer" }));
    expect(
      screen.getByText(/Le mode sans fournisseur sera sélectionné/),
    ).toBeDefined();
    fireEvent.click(
      screen.getByRole("button", { name: "Confirmer la suppression" }),
    );
    await waitFor(() =>
      expect(screen.queryByRole("heading", { name: "Personnel" })).toBeNull(),
    );
    const data = client.getQueryData<ProviderSettings>([
      "ai-settings",
      "actor-a:workspace-a",
    ]);
    expect(data?.selection).toEqual({
      mode: "deterministic",
      connection_id: null,
    });
  });
});
