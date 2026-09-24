// @vitest-environment jsdom
import { webcrypto } from "node:crypto";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter, Route, Routes } from "react-router";
import { api, ApiError } from "@/api/client";
import { companyApi } from "@/api/company";
import { workToolsApi } from "@/api/work-tools";
import { codeObservationsApi } from "@/api/code-observations";
import { fixtureCode } from "@/test-fixtures/code-evidence";
import { fixtureCompany } from "@/test-fixtures/company-context";
import { fixtureSnapshot } from "@/test-fixtures/context-loop";
import {
  fixtureTools,
  fixtureToolConnection,
} from "@/test-fixtures/work-tools";
import { CodeEvidencePage } from "./code-evidence-page";
vi.mock("@/api/client", async (original) => ({
  ...(await original<typeof import("@/api/client")>()),
  api: { snapshot: vi.fn() },
}));
vi.mock("@/api/company", () => ({ companyApi: { overview: vi.fn() } }));
vi.mock("@/api/work-tools", () => ({ workToolsApi: { settings: vi.fn() } }));
vi.mock("@/api/code-observations", () => ({
  codeObservationsApi: { list: vi.fn(), detail: vi.fn(), read: vi.fn() },
}));
let client: QueryClient;
beforeEach(() => {
  vi.resetAllMocks();
  vi.stubGlobal("crypto", webcrypto);
  client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  vi.mocked(api.snapshot).mockResolvedValue(fixtureSnapshot);
  vi.mocked(companyApi.overview).mockResolvedValue(fixtureCompany);
  vi.mocked(workToolsApi.settings).mockResolvedValue({
    ...fixtureTools,
    connections: [{ ...fixtureToolConnection, provider: "github" }],
  });
  vi.mocked(codeObservationsApi.list).mockResolvedValue({
    items: [],
    limit: 25,
    offset: 0,
  });
  vi.mocked(codeObservationsApi.detail).mockResolvedValue(fixtureCode);
  vi.mocked(codeObservationsApi.read).mockResolvedValue(fixtureCode);
});
afterEach(() => {
  cleanup();
  client.clear();
  vi.unstubAllGlobals();
});
function openPage(query = "") {
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[`/projects/fixture-project/code${query}`]}>
        <Routes>
          <Route
            path="projects/:projectId/code"
            element={<CodeEvidencePage />}
          />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}
describe("targeted source read", () => {
  it("retries an unconfirmed read with the same identity and exact scope", async () => {
    vi.mocked(codeObservationsApi.read).mockRejectedValueOnce(
      new ApiError("Délai dépassé", 504),
    );
    openPage();
    fireEvent.click(
      await screen.findByRole("button", {
        name: "Observer des fichiers GitHub",
      }),
    );
    fireEvent.change(
      await screen.findByLabelText("Dépôt (organisation/dépôt)"),
      { target: { value: fixtureCode.corpus.repository } },
    );
    fireEvent.change(screen.getByLabelText("Commit exact"), {
      target: { value: fixtureCode.corpus.commit_sha },
    });
    fireEvent.change(
      screen.getByLabelText("Fichiers à observer, un chemin par ligne"),
      { target: { value: "src/fixture.ts" } },
    );
    fireEvent.click(
      screen.getByRole("button", {
        name: "Lire et conserver cette observation",
      }),
    );
    await screen.findByText("La lecture n’a pas pu être confirmée");
    fireEvent.click(
      screen.getByRole("button", {
        name: "Lire et conserver cette observation",
      }),
    );
    await screen.findByRole("heading", { name: "fiction/example" });
    const calls = vi.mocked(codeObservationsApi.read).mock.calls;
    expect(calls).toHaveLength(2);
    expect(calls[1]).toEqual(calls[0]);
    expect(calls[0].slice(0, 2)).toEqual([
      "fixture-project",
      {
        connection_id: "fixture-tool",
        repository: "fiction/example",
        commit_sha: fixtureCode.corpus.commit_sha,
        paths: ["src/fixture.ts"],
      },
    ]);
  });
  it("does not show an observation belonging to another project", async () => {
    vi.mocked(codeObservationsApi.detail).mockResolvedValue({
      ...fixtureCode,
      corpus: { ...fixtureCode.corpus, project_id: "another-project" },
    });
    openPage("?observation=fixture-corpus");
    await screen.findByText("Observation hors de ce projet");
    expect(
      screen.queryByRole("heading", { name: "fiction/example" }),
    ).toBeNull();
  });
  it("keeps viewer access read-only", async () => {
    vi.mocked(companyApi.overview).mockResolvedValue({
      ...fixtureCompany,
      workspace: { ...fixtureCompany.workspace, role: "viewer" },
    });
    openPage();
    await screen.findByRole("heading", { name: "Preuves de code" });
    expect(
      screen.queryByRole("button", { name: "Observer des fichiers GitHub" }),
    ).toBeNull();
    await waitFor(() =>
      expect(codeObservationsApi.read).not.toHaveBeenCalled(),
    );
  });
});
