// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router";
import { ApiError } from "@/api/client";
import { companyControlsApi } from "@/api/company-controls";
import { setRequestIdentity } from "@/api/request-context";
import { fixtureSnapshot } from "@/test-fixtures/context-loop";
import { CompanyDataControls } from "./data-controls";
vi.mock("@/api/company-controls", () => ({
  companyControlsApi: {
    archivedProjects: vi.fn(),
    archiveProject: vi.fn(),
    exportData: vi.fn(),
  },
}));
let client: QueryClient;
const createObjectURL = vi.fn();
const revokeObjectURL = vi.fn();
let click: ReturnType<typeof vi.spyOn>;
beforeEach(() => {
  vi.resetAllMocks();
  setRequestIdentity("fixture-actor", "fixture-workspace", "fixture-token");
  client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  vi.mocked(companyControlsApi.archivedProjects).mockResolvedValue([
    {
      ...fixtureSnapshot.project,
      public_id: "fixture-archived-project",
      status: "archived",
      name: "[FICTIF] Projet archivé",
    },
  ]);
  vi.mocked(companyControlsApi.archiveProject).mockRejectedValue(
    new ApiError("Réponse interrompue", 504),
  );
  vi.mocked(companyControlsApi.exportData).mockResolvedValue(
    '{"format":"ai-center-company-data-v1","complete":true,"data":{}}',
  );
  vi.stubGlobal(
    "URL",
    class extends URL {
      static createObjectURL = createObjectURL;
      static revokeObjectURL = revokeObjectURL;
    },
  );
  createObjectURL.mockReturnValue("blob:fixture-export");
  click = vi
    .spyOn(HTMLAnchorElement.prototype, "click")
    .mockImplementation(() => {});
});
afterEach(() => {
  cleanup();
  client.clear();
  click.mockRestore();
  vi.unstubAllGlobals();
});
function open() {
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <CompanyDataControls
          workspaceId="fixture-workspace"
          projects={[fixtureSnapshot.project]}
        />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}
describe("company archives and canonical export", () => {
  it("preserves retry identity and sends a separate explicit restore command", async () => {
    open();
    fireEvent.click(screen.getByRole("button", { name: "Archiver ce projet" }));
    await screen.findByText("Le changement d’archive n’a pas pu être confirmé");
    fireEvent.click(screen.getByRole("button", { name: "Réessayer" }));
    await waitFor(() =>
      expect(companyControlsApi.archiveProject).toHaveBeenCalledTimes(2),
    );
    const calls = vi.mocked(companyControlsApi.archiveProject).mock.calls;
    expect(calls[1]).toEqual(calls[0]);
    expect(calls[0].slice(0, 2)).toEqual(["fixture-project", true]);
    await waitFor(() =>
      expect(
        (
          screen.getByRole("button", {
            name: "Restaurer le projet",
          }) as HTMLButtonElement
        ).disabled,
      ).toBe(false),
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Restaurer le projet" }),
    );
    await waitFor(() =>
      expect(companyControlsApi.archiveProject).toHaveBeenCalledTimes(3),
    );
    expect(
      vi.mocked(companyControlsApi.archiveProject).mock.calls[2].slice(0, 2),
    ).toEqual(["fixture-archived-project", false]);
    expect(
      screen.getByText(/restauration ne réactive pas les anciens packs/),
    ).toBeDefined();
  });
  it("downloads only canonical export bytes on demand and releases its temporary URL", async () => {
    open();
    expect(companyControlsApi.exportData).not.toHaveBeenCalled();
    fireEvent.click(
      screen.getByRole("button", { name: "Exporter les données" }),
    );
    await screen.findByText(/Téléchargement de l’export complet demandé/);
    expect(createObjectURL).toHaveBeenCalledWith(expect.any(Blob));
    expect(click).toHaveBeenCalledOnce();
    await waitFor(() =>
      expect(revokeObjectURL).toHaveBeenCalledWith("blob:fixture-export"),
    );
  });
  it("does not download data returned after switching workspace", async () => {
    let finish!: (text: string) => void;
    vi.mocked(companyControlsApi.exportData).mockImplementation(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    open();
    fireEvent.click(
      screen.getByRole("button", { name: "Exporter les données" }),
    );
    await waitFor(() => expect(finish).toBeDefined());
    setRequestIdentity("fixture-actor", "fixture-workspace-b", "fixture-token");
    finish("[FICTIF] Other workspace private export");
    await waitFor(() =>
      expect(
        (
          screen.getByRole("button", {
            name: "Exporter les données",
          }) as HTMLButtonElement
        ).disabled,
      ).toBe(false),
    );
    expect(createObjectURL).not.toHaveBeenCalled();
    expect(click).not.toHaveBeenCalled();
  });
});
