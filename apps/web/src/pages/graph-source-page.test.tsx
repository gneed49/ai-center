// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter, Route, Routes } from "react-router";
import { companyApi } from "@/api/company";
import { GraphSourcePage } from "./graph-source-page";
import type { GraphSourceView } from "@/api/company-types";
vi.mock("@/api/company", () => ({ companyApi: { source: vi.fn() } }));
const project = "10000000-0000-0000-0000-000000000001";
const id = "20000000-0000-0000-0000-000000000001";
const fixture: GraphSourceView = {
  public_id: id,
  project_public_id: project,
  project_name: "[FICTIF] Portail",
  scope_kind: "project",
  kind: "knowledge",
  title: "[FICTIF] Règle historique",
  status: "superseded",
  version_number: 1,
  recorded_at: "2026-09-22T10:00:00Z",
  content: {
    statement:
      "[FICTIF] Ancienne règle exacte <script>window.bad=true</script>",
    rationale: "Conserver la source citée.",
  },
  links: [
    {
      label: "[FICTIF] Discussion d’origine",
      app_path: `/projects/${project}/sessions/${id}`,
    },
    { label: "Lien refusé", app_path: "//evil.invalid" },
  ],
};
afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});
function open(record = fixture) {
  vi.mocked(companyApi.source).mockResolvedValue(record);
  render(
    <QueryClientProvider
      client={
        new QueryClient({
          defaultOptions: { queries: { retry: false, gcTime: 0 } },
        })
      }
    >
      <MemoryRouter
        initialEntries={[`/projects/${project}/sources/${record.kind}/${id}`]}
      >
        <Routes>
          <Route
            path="projects/:projectId/sources/:kind/:sourceId"
            element={<GraphSourcePage />}
          />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}
describe("source exacte du graphe", () => {
  it("conserve l’ancienne version, sa provenance et échappe le contenu", async () => {
    open();
    expect(
      await screen.findByRole("heading", { name: fixture.title }),
    ).toBeTruthy();
    expect(screen.getByText("Version 1")).toBeTruthy();
    expect(
      screen.getByText(/Vous consultez une version historique/),
    ).toBeTruthy();
    expect(
      screen
        .getByRole("link", { name: "[FICTIF] Discussion d’origine" })
        .getAttribute("href"),
    ).toBe(fixture.links[0].app_path);
    expect(screen.queryByRole("link", { name: "Lien refusé" })).toBeNull();
    expect(document.querySelector("script")).toBeNull();
    expect(companyApi.source).toHaveBeenCalledWith(project, "knowledge", id);
  });
  it("refuse une réponse concernant un autre projet sans afficher son contenu", async () => {
    open({
      ...fixture,
      project_public_id: "30000000-0000-0000-0000-000000000001",
    });
    expect(
      await screen.findByText(/La source reçue ne correspond pas/),
    ).toBeTruthy();
    expect(screen.queryByRole("heading", { name: fixture.title })).toBeNull();
  });
  it("ouvre une tâche en lecture seule avec son contexte lié", async () => {
    open({
      ...fixture,
      kind: "task",
      version_number: null,
      status: "ready",
      links: [
        {
          label: "Contexte v1",
          app_path: `/projects/${project}/sources/context_pack/${id}`,
        },
      ],
    });
    expect(
      await screen.findByRole("link", { name: "Contexte v1" }),
    ).toBeTruthy();
    expect(screen.queryByText("Version 1")).toBeNull();
    expect(screen.queryByRole("button", { name: /Exécuter/ })).toBeNull();
  });
});
