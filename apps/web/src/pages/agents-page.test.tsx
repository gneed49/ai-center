// @vitest-environment jsdom
import { beforeEach, afterEach, describe, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter, Route, Routes, useLocation } from "react-router";
import { api, ApiError } from "@/api/client";
import { companyApi } from "@/api/company";
import { fixtureCompany } from "@/test-fixtures/company-context";
import { fixtureSession, fixtureSnapshot } from "@/test-fixtures/context-loop";
import { AgentsPage } from "./agents-page";

vi.mock("@/api/client", async (original) => ({
  ...(await original<typeof import("@/api/client")>()),
  api: { snapshot: vi.fn(), createSession: vi.fn() },
}));
vi.mock("@/api/company", () => ({ companyApi: { overview: vi.fn() } }));
let client: QueryClient;
beforeEach(() => {
  vi.resetAllMocks();
  client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  vi.mocked(companyApi.overview).mockResolvedValue(fixtureCompany);
  vi.mocked(api.snapshot).mockResolvedValue(fixtureSnapshot);
  vi.mocked(api.createSession).mockResolvedValue(fixtureSession);
});
afterEach(() => {
  cleanup();
  client.clear();
});
function Destination() {
  return <p>Destination {useLocation().pathname}</p>;
}
function openPage(path = "/agents?project=fixture-project") {
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[path]}>
        <Routes>
          <Route path="/agents" element={<AgentsPage />} />
          <Route path="*" element={<Destination />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("scoped agent conversations", () => {
  it("resumes a real existing session without creating another one", async () => {
    openPage();
    fireEvent.click(
      await screen.findByRole("link", { name: /Reprendre la conversation/ }),
    );
    await screen.findByText(
      "Destination /projects/fixture-project/sessions/fixture-session",
    );
    expect(api.createSession).not.toHaveBeenCalled();
  });

  it("opens a company conversation with the company scope and general profile", async () => {
    openPage("/agents");
    fireEvent.click(
      await screen.findByRole("button", { name: "Nouvelle conversation" }),
    );
    await screen.findByText(
      "Destination /projects/fixture-company-scope/sessions/fixture-session",
    );
    expect(api.snapshot).toHaveBeenCalledWith("fixture-company-scope");
    expect(api.createSession).toHaveBeenCalledWith(
      "fixture-company-scope",
      "general",
      undefined,
      expect.any(String),
    );
  });

  it("selects a role from the server catalogue and keeps its command identity on retry", async () => {
    vi.mocked(api.createSession).mockRejectedValue(
      new ApiError("Réponse interrompue", 504),
    );
    openPage();
    fireEvent.click(await screen.findByRole("button", { name: "Commercial" }));
    fireEvent.click(
      await screen.findByRole("button", { name: "Nouvelle conversation" }),
    );
    await screen.findByText("La conversation n’a pas pu être ouverte");
    fireEvent.click(
      screen.getByRole("button", { name: "Nouvelle conversation" }),
    );
    await waitFor(() => expect(api.createSession).toHaveBeenCalledTimes(2));
    const calls = vi.mocked(api.createSession).mock.calls;
    expect(calls[0].slice(0, 3)).toEqual([
      "fixture-project",
      "sales",
      undefined,
    ]);
    expect(calls[1]).toEqual(calls[0]);
  });

  it("requires the existing product handoff to open a technical session", async () => {
    openPage();
    fireEvent.click(
      await screen.findByRole("button", {
        name: "Lead technique",
      }),
    );
    const handoff = await screen.findByRole("link", {
      name: /Préparer le relais technique/,
    });
    expect(handoff.getAttribute("href")).toBe(
      "/projects/fixture-project/handoff",
    );
    expect(
      screen.queryByRole("button", { name: "Nouvelle conversation" }),
    ).toBeNull();
    expect(api.createSession).not.toHaveBeenCalled();
  });

  it("prepares a pack targeted to development using the server node key", async () => {
    openPage("/agents?project=fixture-project&agent=dev");
    const handoff = await screen.findByRole("link", {
      name: /Préparer le relais technique/,
    });
    expect(handoff.getAttribute("href")).toBe(
      "/projects/fixture-project/handoff?target=dev",
    );
    expect(api.createSession).not.toHaveBeenCalled();
  });

  it("keeps reader access read only and rejects an inaccessible scope", async () => {
    vi.mocked(companyApi.overview).mockResolvedValue({
      ...fixtureCompany,
      workspace: { ...fixtureCompany.workspace, role: "viewer" },
    });
    openPage();
    await screen.findByText(/Votre accès en lecture/);
    expect(
      screen.queryByRole("button", { name: "Nouvelle conversation" }),
    ).toBeNull();
    expect(
      screen.getByRole("link", { name: /Reprendre la conversation/ }),
    ).toBeDefined();
  });

  it("does not silently use company data for an inaccessible project", async () => {
    openPage("/agents?project=other-company-project");
    await screen.findByText("Projet inaccessible");
    expect(api.snapshot).toHaveBeenCalledWith("other-company-project");
    expect(api.createSession).not.toHaveBeenCalled();
  });
});

it("opens archived conversation history from snapshot nodes without creating a new session", async () => {
  vi.mocked(companyApi.overview).mockResolvedValue({
    ...fixtureCompany,
    projects: [],
    agents: [],
  });
  vi.mocked(api.snapshot).mockResolvedValue({
    ...fixtureSnapshot,
    project: { ...fixtureSnapshot.project, status: "archived" },
    nodes: [
      {
        public_id: "fixture-node",
        node_key: "product",
        title: "Produit",
        description: "[FICTIF]",
        summary: "",
        profile_key: "product-agent",
        profile_name: "Agent Produit",
        scope_kind: "product",
      },
    ],
  });
  openPage();
  fireEvent.click(
    await screen.findByRole("link", { name: /Consulter la conversation/ }),
  );
  await screen.findByText(
    "Destination /projects/fixture-project/sessions/fixture-session",
  );
  expect(api.createSession).not.toHaveBeenCalled();
});
