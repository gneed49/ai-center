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
import { companyApi } from "@/api/company";
import { companyControlsApi } from "@/api/company-controls";
import { ApiError } from "@/api/client";
import { fixtureCompany } from "@/test-fixtures/company-context";
import { fixtureSnapshot } from "@/test-fixtures/context-loop";
import { ProjectRename } from "./project-rename";
vi.mock("@/api/company", () => ({ companyApi: { overview: vi.fn() } }));
vi.mock("@/api/company-controls", () => ({
  companyControlsApi: { renameProject: vi.fn() },
}));
let client: QueryClient;
beforeEach(() => {
  vi.resetAllMocks();
  client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  vi.mocked(companyApi.overview).mockResolvedValue(fixtureCompany);
  vi.mocked(companyControlsApi.renameProject).mockRejectedValue(
    new ApiError("Réponse interrompue", 504),
  );
});
afterEach(() => {
  cleanup();
  client.clear();
});
const page = (updatedAt = fixtureSnapshot.project.updated_at) => (
  <QueryClientProvider client={client}>
    <ProjectRename
      project={{ ...fixtureSnapshot.project, updated_at: updatedAt }}
    />
  </QueryClientProvider>
);
it("retries against the captured exact revision despite a refreshed project", async () => {
  const view = render(page());
  fireEvent.click(
    await screen.findByRole("button", { name: "Renommer le projet" }),
  );
  fireEvent.change(screen.getByLabelText("Nouveau nom du projet"), {
    target: { value: "[FICTIF] Nouveau nom" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Enregistrer le nom" }));
  await screen.findByText("Le renommage n’a pas pu être confirmé");
  view.rerender(page("2026-09-21T12:00:00Z"));
  fireEvent.click(screen.getByRole("button", { name: "Enregistrer le nom" }));
  await waitFor(() =>
    expect(companyControlsApi.renameProject).toHaveBeenCalledTimes(2),
  );
  const calls = vi.mocked(companyControlsApi.renameProject).mock.calls;
  expect(calls[1]).toEqual(calls[0]);
  expect(calls[0][1]).toEqual({
    name: "[FICTIF] Nouveau nom",
    expected_updated_at: fixtureSnapshot.project.updated_at,
  });
});
it("does not offer renaming to a reader", async () => {
  vi.mocked(companyApi.overview).mockResolvedValue({
    ...fixtureCompany,
    workspace: { ...fixtureCompany.workspace, role: "viewer" },
  });
  render(page());
  await waitFor(() => expect(companyApi.overview).toHaveBeenCalled());
  expect(
    screen.queryByRole("button", { name: "Renommer le projet" }),
  ).toBeNull();
});
