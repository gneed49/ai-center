// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { api } from "@/api/client";
import { setRequestIdentity } from "@/api/request-context";
import { downloadContextPack } from "@/lib/context-pack-export";
import { fixturePack, fixtureSnapshot } from "@/test-fixtures/context-loop";
import { ContextPackExport } from "./context-pack-export";

vi.mock("@/api/client", () => ({
  api: { contextPack: vi.fn(), snapshot: vi.fn(), exportContextPack: vi.fn() },
}));
vi.mock("@/lib/context-pack-export", () => ({ downloadContextPack: vi.fn() }));
const writeText = vi.fn();

beforeEach(() => {
  vi.resetAllMocks();
  setRequestIdentity("fixture-actor", "fixture-workspace", "fixture-token");
  vi.mocked(api.contextPack).mockResolvedValue(fixturePack);
  vi.mocked(api.snapshot).mockResolvedValue(fixtureSnapshot);
  vi.mocked(api.exportContextPack).mockResolvedValue(
    "[FICTIF] canonical export bytes",
  );
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText },
  });
  writeText.mockResolvedValue(undefined);
});
afterEach(cleanup);
const page = (pack = fixturePack) => <ContextPackExport pack={pack} />;

describe("context pack export actions", () => {
  it("copies canonical Markdown with explicit provenance and version", async () => {
    render(page());
    expect(screen.getByText(/version 2 · contexte source v4/)).toBeDefined();
    fireEvent.click(screen.getByRole("button", { name: "Copier le contexte" }));
    await screen.findByText(
      "Contexte Markdown copié avec sa version et ses sources.",
    );
    expect(api.exportContextPack).toHaveBeenCalledWith(
      fixturePack.public_id,
      "markdown",
    );
    expect(writeText).toHaveBeenCalledWith("[FICTIF] canonical export bytes");
    expect(downloadContextPack).not.toHaveBeenCalled();
  });

  it.each(["markdown", "json"] as const)(
    "downloads canonical %s bytes without rewriting sources",
    async (format) => {
      render(page());
      fireEvent.click(
        screen.getByRole("button", {
          name: format === "json" ? "Télécharger JSON" : "Télécharger Markdown",
        }),
      );
      await screen.findByText(
        "Téléchargement demandé. Conservez ce fichier avec sa version.",
      );
      expect(api.exportContextPack).toHaveBeenCalledWith(
        fixturePack.public_id,
        format,
      );
      expect(downloadContextPack).toHaveBeenCalledWith(
        fixturePack,
        format,
        "[FICTIF] canonical export bytes",
      );
    },
  );

  it("blocks a pack already known to be obsolete", () => {
    render(page({ ...fixturePack, status: "stale" }));
    expect(screen.getByRole("alert").textContent).toContain(
      "Ce pack est obsolète",
    );
    fireEvent.click(screen.getByRole("button", { name: "Copier le contexte" }));
    expect(api.contextPack).not.toHaveBeenCalled();
    expect(api.exportContextPack).not.toHaveBeenCalled();
  });

  it("checks server freshness at the moment of export", async () => {
    vi.mocked(api.contextPack).mockResolvedValue({
      ...fixturePack,
      status: "stale",
    });
    render(page());
    fireEvent.click(screen.getByRole("button", { name: "Télécharger JSON" }));
    await screen.findByText(/Ce pack est obsolète/);
    expect(api.exportContextPack).not.toHaveBeenCalled();
    expect(downloadContextPack).not.toHaveBeenCalled();
  });

  it("does not copy or download a response from an abandoned workspace", async () => {
    let finish!: (value: string) => void;
    vi.mocked(api.exportContextPack).mockImplementation(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    render(page());
    fireEvent.click(screen.getByRole("button", { name: "Copier le contexte" }));
    await waitFor(() => expect(finish).toBeDefined());
    setRequestIdentity(
      "fixture-actor",
      "fixture-other-workspace",
      "fixture-token",
    );
    finish("private previous workspace data");
    await waitFor(() =>
      expect(screen.queryByText(/préparation de l’export/)).toBeNull(),
    );
    expect(writeText).not.toHaveBeenCalled();
    expect(downloadContextPack).not.toHaveBeenCalled();
  });

  it("explains clipboard failures and keeps the download available", async () => {
    writeText.mockRejectedValue(new Error("Copie refusée par le navigateur"));
    render(page());
    fireEvent.click(screen.getByRole("button", { name: "Copier le contexte" }));
    await screen.findByText("Copie refusée par le navigateur");
    expect(
      (
        screen.getByRole("button", {
          name: "Télécharger Markdown",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(false);
    expect(screen.queryByText(/Contexte Markdown copié/)).toBeNull();
  });
});
