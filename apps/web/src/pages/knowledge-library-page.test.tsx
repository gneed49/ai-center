// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router";
import { companyApi } from "@/api/company";
import { fixtureCompany } from "@/test-fixtures/company-context";
import { KnowledgeLibraryPage } from "./knowledge-library-page";
import type { KnowledgeLibraryItem } from "@/api/company-types";
vi.mock("@/api/company", () => ({
  companyApi: { overview: vi.fn(), knowledge: vi.fn() },
}));
const items: KnowledgeLibraryItem[] = Array.from(
  { length: 31 },
  (_, index) => ({
    public_id: `source-${index}`,
    project_public_id: "fixture-project",
    project_name: "[FICTIF] Projet",
    project_status: "archived",
    scope_kind: "project",
    title: `[FICTIF] Règle ${index}`,
    excerpt: "[FICTIF] Une décision conservée.",
    entry_type: "business_rule",
    status: "confirmed",
    version_number: 1,
    recorded_at: "2026-09-22T10:00:00Z",
  }),
);
beforeEach(() => {
  vi.mocked(companyApi.overview).mockResolvedValue(fixtureCompany);
  vi.mocked(companyApi.knowledge).mockImplementation(async (filters) => ({
    items: items.slice(filters.offset ?? 0, (filters.offset ?? 0) + 25),
    total: 31,
    limit: 25,
    offset: filters.offset ?? 0,
  }));
});
afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});
function open() {
  render(
    <QueryClientProvider
      client={
        new QueryClient({
          defaultOptions: { queries: { retry: false, gcTime: 0 } },
        })
      }
    >
      <MemoryRouter initialEntries={["/knowledge?project=fixture-project"]}>
        <KnowledgeLibraryPage />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}
it("permet d’ouvrir une ancienne connaissance au-delà de la première page", async () => {
  open();
  await screen.findByRole("link", { name: "[FICTIF] Règle 0" });
  expect(screen.queryByRole("link", { name: "[FICTIF] Règle 30" })).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Page suivante" }));
  const old = await screen.findByRole("link", { name: "[FICTIF] Règle 30" });
  expect(old.getAttribute("href")).toBe(
    "/projects/fixture-project/sources/knowledge/source-30",
  );
  expect(
    screen
      .getByRole("button", { name: "Page suivante" })
      .hasAttribute("disabled"),
  ).toBe(true);
  expect(screen.getAllByText(/Projet archivé/).length).toBe(6);
  fireEvent.click(screen.getByLabelText("Inclure les versions historiques"));
  await waitFor(() =>
    expect(companyApi.knowledge).toHaveBeenLastCalledWith({
      project_id: "fixture-project",
      q: "",
      history: true,
      offset: 0,
    }),
  );
});
it("préserve le projet et revient à la première page après une recherche", async () => {
  open();
  await screen.findByRole("link", { name: "[FICTIF] Règle 0" });
  fireEvent.click(screen.getByRole("button", { name: "Page suivante" }));
  await screen.findByRole("link", { name: "[FICTIF] Règle 30" });
  fireEvent.change(screen.getByLabelText("Rechercher une connaissance"), {
    target: { value: "  règle archivée  " },
  });
  fireEvent.click(screen.getByRole("button", { name: "Rechercher" }));
  await waitFor(() =>
    expect(companyApi.knowledge).toHaveBeenLastCalledWith({
      project_id: "fixture-project",
      q: "règle archivée",
      history: false,
      offset: 0,
    }),
  );
});
