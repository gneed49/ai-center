// @vitest-environment jsdom
import { beforeEach, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter, Route, Routes } from "react-router";
import { api } from "@/api/client";
import { companyApi } from "@/api/company";
import { fixtureCompany } from "@/test-fixtures/company-context";
import { AppShell } from "./app-shell";

const auth = vi.hoisted(() => ({
  enabled: true,
  workspaceId: "fixture-workspace",
  session: { user: { email: "fixture@example.invalid" } },
  selectWorkspace: vi.fn(),
  client: { auth: { signOut: vi.fn() } },
}));
vi.mock("@/auth/auth-context", () => ({ useAuth: () => auth }));
vi.mock("@/api/client", () => ({ api: { workspaces: vi.fn() } }));
vi.mock("@/api/company", () => ({ companyApi: { overview: vi.fn() } }));
let client: QueryClient;
beforeEach(() => {
  vi.resetAllMocks();
  client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  vi.mocked(companyApi.overview).mockResolvedValue(fixtureCompany);
  vi.mocked(api.workspaces).mockResolvedValue([
    fixtureCompany.workspace,
    {
      public_id: "fixture-workspace-b",
      name: "[FICTIF] Autre entreprise",
      role: "viewer",
    },
  ]);
});
afterEach(() => {
  cleanup();
  client.clear();
});

function openShell() {
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={["/graph"]}>
        <Routes>
          <Route element={<AppShell />}>
            <Route path="/graph" element={<p>Graphe accessible</p>} />
            <Route path="/" element={<p>Vue d’ensemble accessible</p>} />
          </Route>
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("company navigation and identity", () => {
  it("shows the authenticated identity, actual workspace and concrete navigation", async () => {
    openShell();
    await screen.findByRole("option", { name: "[FICTIF] Atelier Exemple" });
    expect(screen.getByText("fixture@example.invalid")).toBeDefined();
    expect(screen.getByText("Propriétaire")).toBeDefined();
    expect(screen.queryByText("GM")).toBeNull();
    expect(screen.queryByText("Control plane local")).toBeNull();
    expect(
      screen.getByRole("link", { name: "Équipe" }).getAttribute("href"),
    ).toBe("/company");
    expect(
      screen.getByRole("link", { name: "Agents" }).getAttribute("href"),
    ).toBe("/agents");
  });

  it("switches only to an accessible company and returns to its overview", async () => {
    openShell();
    const selector = await screen.findByRole("combobox", {
      name: "Entreprise",
    });
    fireEvent.change(selector, { target: { value: "fixture-workspace-b" } });
    expect(auth.selectWorkspace).toHaveBeenCalledWith("fixture-workspace-b");
    await screen.findByText("Vue d’ensemble accessible");
  });
});
