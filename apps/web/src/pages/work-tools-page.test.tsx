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
import { MemoryRouter } from "react-router";
import { companyApi } from "@/api/company";
import { workToolsApi } from "@/api/work-tools";
import { setRequestIdentity } from "@/api/request-context";
import { ApiError } from "@/api/client";
import { fixtureCompany } from "@/test-fixtures/company-context";
import {
  fixtureToolConnection,
  fixtureTools,
} from "@/test-fixtures/work-tools";
import { WorkToolsPage } from "./work-tools-page";
const auth = vi.hoisted(() => ({
  workspaceId: "fixture-workspace",
  session: { user: { id: "fixture-actor" } },
}));
vi.mock("@/auth/auth-context", () => ({ useAuth: () => auth }));
vi.mock("@/api/company", () => ({ companyApi: { overview: vi.fn() } }));
vi.mock("@/api/work-tools", () => ({
  workToolsApi: {
    settings: vi.fn(),
    save: vi.fn(),
    test: vi.fn(),
    disable: vi.fn(),
  },
}));
let client: QueryClient;
beforeEach(() => {
  vi.resetAllMocks();
  vi.stubGlobal("crypto", webcrypto);
  auth.workspaceId = "fixture-workspace";
  setRequestIdentity(
    "fixture-actor",
    "fixture-workspace",
    "fixture-access-token",
  );
  client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  vi.mocked(companyApi.overview).mockResolvedValue(fixtureCompany);
  vi.mocked(workToolsApi.settings).mockResolvedValue({
    ...fixtureTools,
    connections: [],
  });
  vi.mocked(workToolsApi.save).mockResolvedValue(fixtureToolConnection);
  vi.mocked(workToolsApi.test).mockResolvedValue({
    ok: true,
    code: "credential_verified",
  });
});
afterEach(() => {
  cleanup();
  client.clear();
  vi.unstubAllGlobals();
});
const page = () => (
  <QueryClientProvider client={client}>
    <MemoryRouter>
      <WorkToolsPage />
    </MemoryRouter>
  </QueryClientProvider>
);
describe("shared work tool connections", () => {
  it("retries a credential save identically and keeps its secret out of query and mutation caches", async () => {
    vi.mocked(workToolsApi.save).mockRejectedValueOnce(
      new ApiError("Délai dépassé", 504),
    );
    render(page());
    fireEvent.click(
      await screen.findByRole("button", { name: "Ajouter une connexion" }),
    );
    fireEvent.change(
      screen.getByRole("textbox", { name: "Nom pour l’équipe" }),
      { target: { value: "[FICTIF] Documentation" } },
    );
    const secret = screen.getByLabelText("Clé d’accès");
    fireEvent.change(secret, { target: { value: "fixture-only-private-key" } });
    fireEvent.click(
      screen.getByRole("button", { name: "Enregistrer la connexion" }),
    );
    await screen.findByRole("alert");
    fireEvent.click(
      screen.getByRole("button", { name: "Enregistrer la connexion" }),
    );
    await screen.findByText(
      "Connexion enregistrée. Aucune publication n’a été lancée.",
    );
    const calls = vi.mocked(workToolsApi.save).mock.calls;
    expect(calls[1]).toEqual(calls[0]);
    expect(client.getMutationCache().getAll()).toHaveLength(0);
    expect(
      JSON.stringify(
        client
          .getQueryCache()
          .getAll()
          .map((query) => query.state.data),
      ),
    ).not.toContain("fixture-only-private-key");
    expect((secret as HTMLInputElement).value).toBe("");
    expect(workToolsApi.test).not.toHaveBeenCalled();
  });
  it("clears an unfinished credential when the identity boundary changes", async () => {
    const view = render(page());
    fireEvent.click(
      await screen.findByRole("button", { name: "Ajouter une connexion" }),
    );
    fireEvent.change(screen.getByLabelText("Clé d’accès"), {
      target: { value: "fixture-only-private-key" },
    });
    auth.workspaceId = "fixture-workspace-b";
    setRequestIdentity(
      "fixture-actor",
      "fixture-workspace-b",
      "fixture-access-token",
    );
    view.rerender(page());
    await screen.findByRole("button", { name: "Ajouter une connexion" });
    expect(screen.queryByLabelText("Clé d’accès")).toBeNull();
    expect(workToolsApi.save).not.toHaveBeenCalled();
  });
  it("tests only on the owner's explicit action and does not equate the test with destination permissions", async () => {
    vi.mocked(workToolsApi.settings).mockResolvedValue(fixtureTools);
    render(page());
    await screen.findByRole("heading", { name: "[FICTIF] Documentation" });
    expect(workToolsApi.test).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Tester l’accès" }));
    await screen.findByText(/L’outil reconnaît cette clé/);
    await waitFor(() =>
      expect(workToolsApi.test).toHaveBeenCalledWith(
        "fixture-tool",
        expect.any(String),
      ),
    );
  });
});
