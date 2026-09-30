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
import { MemoryRouter, Route, Routes, useLocation } from "react-router";
import { api } from "@/api/client";
import { teamApi } from "@/api/team";
import { setRequestIdentity } from "@/api/request-context";
import { JoinCompanyPage } from "./join-company-page";
import App from "@/App";
const auth = vi.hoisted(() => ({
  enabled: true,
  loading: false,
  workspaceId: null as string | null,
  session: null as null | { user: { id: string; email: string } },
  client: { auth: { signInWithOtp: vi.fn() } },
  selectWorkspace: vi.fn(),
  error: null,
}));
vi.mock("@/auth/auth-context", () => ({ useAuth: () => auth }));
vi.mock("@/api/client", async (original) => ({
  ...(await original<typeof import("@/api/client")>()),
  api: { workspaces: vi.fn() },
}));
vi.mock("@/api/team", () => ({
  teamApi: { preview: vi.fn(), accept: vi.fn() },
}));
let client: QueryClient;
const syntheticToken = "c".repeat(64);
const preview = {
  public_id: "fixture-invitation",
  workspace_public_id: "fixture-workspace",
  company_name: "[FICTIF] Atelier Exemple",
  role: "editor" as const,
  expires_at: "2026-09-25T12:00:00Z",
};
beforeEach(() => {
  vi.resetAllMocks();
  auth.session = null;
  auth.workspaceId = null;
  auth.loading = false;
  setRequestIdentity(null, null, null);
  client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  vi.mocked(teamApi.preview).mockResolvedValue(preview);
  vi.mocked(teamApi.accept).mockResolvedValue({
    public_id: "fixture-workspace",
    name: "[FICTIF] Atelier Exemple",
    role: "editor",
  });
  vi.mocked(api.workspaces).mockResolvedValue([]);
  auth.client.auth.signInWithOtp.mockResolvedValue({ error: null });
});
afterEach(() => {
  cleanup();
  client.clear();
});
function Destination() {
  const location = useLocation();
  return (
    <p>
      Destination {location.pathname}
      {location.hash}
    </p>
  );
}
function openPage({
  hash = `#token=${syntheticToken}`,
  app = false,
}: { hash?: string; app?: boolean } = {}) {
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[`/join/fixture-invitation${hash}`]}>
        {app ? (
          <App />
        ) : (
          <Routes>
            <Route path="join/:invitationId" element={<JoinCompanyPage />} />
            <Route path="*" element={<Destination />} />
          </Routes>
        )}
      </MemoryRouter>
    </QueryClientProvider>,
  );
}
describe("join company before workspace selection", () => {
  it("bypasses login gating to preview the company, then requests a secret-free sign-in link", async () => {
    const storage = vi.spyOn(Storage.prototype, "setItem");
    openPage({ app: true });
    await screen.findByRole("heading", { name: "[FICTIF] Atelier Exemple" });
    expect(screen.getByText("Collaborateur")).toBeDefined();
    fireEvent.change(
      screen.getByRole("textbox", { name: "Votre adresse e-mail" }),
      { target: { value: "fixture@example.invalid" } },
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Recevoir mon lien de connexion" }),
    );
    await screen.findByText(/Gardez cet onglet ouvert/);
    expect(auth.client.auth.signInWithOtp).toHaveBeenCalledWith({
      email: "fixture@example.invalid",
      options: {
        shouldCreateUser: true,
        emailRedirectTo: `${window.location.origin}/auth/callback`,
      },
    });
    expect(teamApi.accept).not.toHaveBeenCalled();
    expect(storage).not.toHaveBeenCalled();
    storage.mockRestore();
    expect(
      JSON.stringify(
        client
          .getQueryCache()
          .getAll()
          .map((query) => query.queryKey),
      ),
    ).not.toContain(syntheticToken);
  });
  it("accepts explicitly after authentication without a selected workspace and removes the fragment on navigation", async () => {
    auth.session = {
      user: { id: "fixture-actor", email: "fixture@example.invalid" },
    };
    setRequestIdentity("fixture-actor", null, "fixture-access-token");
    openPage();
    fireEvent.change(
      await screen.findByRole("textbox", { name: "Votre nom dans l’équipe" }),
      { target: { value: "[FICTIF] Nadia" } },
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Accepter l’invitation" }),
    );
    await screen.findByText("Destination /");
    expect(teamApi.accept).toHaveBeenCalledWith(
      "fixture-invitation",
      syntheticToken,
      "[FICTIF] Nadia",
    );
    expect(auth.selectWorkspace).toHaveBeenCalledWith("fixture-workspace");
  });
  it("opens existing membership without consuming another invitation", async () => {
    auth.session = {
      user: { id: "fixture-actor", email: "fixture@example.invalid" },
    };
    vi.mocked(api.workspaces).mockResolvedValue([
      {
        public_id: "fixture-workspace",
        name: "[FICTIF] Atelier Exemple",
        role: "viewer",
      },
    ]);
    openPage();
    fireEvent.click(
      await screen.findByRole("button", {
        name: "Ouvrir [FICTIF] Atelier Exemple",
      }),
    );
    await screen.findByText("Destination /");
    expect(teamApi.accept).not.toHaveBeenCalled();
  });
  it("does not contact preview for an incomplete secret", async () => {
    openPage({ hash: "#token=short" });
    await screen.findByText("Le lien d’invitation est incomplet");
    expect(teamApi.preview).not.toHaveBeenCalled();
  });
  it("keeps name and token available to retry an ambiguous acceptance", async () => {
    auth.session = {
      user: { id: "fixture-actor", email: "fixture@example.invalid" },
    };
    vi.mocked(teamApi.accept).mockRejectedValue(
      new Error("Réponse interrompue"),
    );
    openPage();
    fireEvent.change(
      await screen.findByRole("textbox", { name: "Votre nom dans l’équipe" }),
      { target: { value: "[FICTIF] Nadia" } },
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Accepter l’invitation" }),
    );
    await screen.findByText("L’accès n’a pas pu être confirmé");
    fireEvent.click(
      screen.getByRole("button", { name: "Accepter l’invitation" }),
    );
    await waitFor(() => expect(teamApi.accept).toHaveBeenCalledTimes(2));
    expect(vi.mocked(teamApi.accept).mock.calls[1]).toEqual(
      vi.mocked(teamApi.accept).mock.calls[0],
    );
  });
});
