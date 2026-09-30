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
import { teamApi } from "@/api/team";
import type { TeamInvitation, TeamMember } from "@/api/team-types";
import { companyApi } from "@/api/company";
import { ApiError } from "@/api/client";
import { TeamAccess } from "./team-access";
const auth = vi.hoisted(() => ({
  session: { user: { id: "fixture-actor", email: "fixture@example.invalid" } },
}));
vi.mock("@/auth/auth-context", () => ({ useAuth: () => auth }));
vi.mock("@/api/team", () => ({
  teamApi: {
    members: vi.fn(),
    profile: vi.fn(),
    invitations: vi.fn(),
    createInvitation: vi.fn(),
    revokeInvitation: vi.fn(),
  },
}));
vi.mock("@/api/company", () => ({ companyApi: { updateMember: vi.fn() } }));
const member: TeamMember = {
  public_id: "fixture-member",
  actor_id: "fixture-actor",
  role: "owner",
  invitation_status: "accepted",
  accepted_at: "2026-09-21T10:00:00Z",
  display_name: "[FICTIF] Nadia",
};
let client: QueryClient;
let invitations: TeamInvitation[];
const writeText = vi.fn();
beforeEach(() => {
  vi.resetAllMocks();
  vi.stubGlobal("crypto", webcrypto);
  client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  invitations = [];
  vi.mocked(teamApi.members).mockResolvedValue([member]);
  vi.mocked(teamApi.profile).mockImplementation(async (name) => ({
    ...member,
    display_name: name,
  }));
  vi.mocked(teamApi.invitations).mockImplementation(async () => invitations);
  vi.mocked(teamApi.createInvitation).mockImplementation(async (input) => {
    const created = {
      public_id: input.public_id,
      role: input.role,
      label: input.label,
      status: "pending" as const,
      expires_at: "2026-09-28T10:00:00Z",
      created_at: "2026-09-21T10:00:00Z",
      accepted_at: null,
    };
    invitations = [created];
    return created;
  });
  vi.mocked(teamApi.revokeInvitation).mockImplementation(async (id) => {
    const revoked = {
      ...invitations.find((item) => item.public_id === id)!,
      status: "revoked" as const,
    };
    invitations = [revoked];
    return revoked;
  });
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText },
  });
  writeText.mockResolvedValue(undefined);
});
afterEach(() => {
  cleanup();
  client.clear();
  vi.unstubAllGlobals();
});
function openTeam(role: TeamMember["role"] = "owner") {
  render(
    <QueryClientProvider client={client}>
      <TeamAccess role={role} />
    </QueryClientProvider>,
  );
}
describe("team invitations and member names", () => {
  it("retries creation with the same hash and command, copies the secret only on demand and clears the link after revocation", async () => {
    const storage = vi.spyOn(Storage.prototype, "setItem");
    vi.mocked(teamApi.createInvitation).mockRejectedValueOnce(
      new ApiError("Réponse interrompue", 504),
    );
    openTeam();
    fireEvent.change(
      await screen.findByRole("textbox", { name: "Repère pour votre suivi" }),
      { target: { value: "[FICTIF] Relecture" } },
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Créer le lien d’invitation" }),
    );
    await screen.findByText("L’action n’a pas pu être confirmée");
    fireEvent.click(
      screen.getByRole("button", { name: "Créer le lien d’invitation" }),
    );
    const input = await screen.findByRole("textbox", {
      name: "Lien de cette invitation",
    });
    const url = new URL((input as HTMLInputElement).value);
    const token = new URLSearchParams(url.hash.slice(1)).get("token")!;
    const calls = vi.mocked(teamApi.createInvitation).mock.calls;
    expect(calls[1]).toEqual(calls[0]);
    expect(calls[0][0].token_hash).toMatch(/^[a-f0-9]{64}$/);
    expect(JSON.stringify(calls[0])).not.toContain(token);
    expect(url.search).toBe("");
    expect(writeText).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Copier le lien" }));
    await screen.findByText(
      "Lien copié. Transmettez-le à la personne invitée.",
    );
    expect(writeText).toHaveBeenCalledWith(url.toString());
    expect(storage).not.toHaveBeenCalled();
    storage.mockRestore();
    fireEvent.click(
      await screen.findByRole("button", { name: "Révoquer l’invitation" }),
    );
    await waitFor(() =>
      expect(
        screen.queryByRole("textbox", { name: "Lien de cette invitation" }),
      ).toBeNull(),
    );
    expect(teamApi.revokeInvitation).toHaveBeenCalledWith(
      calls[0][0].public_id,
      expect.any(String),
    );
  });
  it("lets a reader change their own name while hiding invitations and access controls", async () => {
    vi.mocked(teamApi.members).mockResolvedValue([
      { ...member, role: "viewer" },
    ]);
    openTeam("viewer");
    await screen.findByText("[FICTIF] Nadia");
    expect(
      screen.queryByRole("button", { name: "Créer le lien d’invitation" }),
    ).toBeNull();
    expect(screen.queryByRole("combobox", { name: /Rôle de/ })).toBeNull();
    expect(teamApi.invitations).not.toHaveBeenCalled();
    fireEvent.change(
      screen.getByRole("textbox", { name: "Votre nom dans l’équipe" }),
      { target: { value: "[FICTIF] Nadia Lectrice" } },
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Enregistrer mon nom" }),
    );
    await screen.findByText("Votre nom est enregistré.");
    expect(teamApi.profile).toHaveBeenCalledWith(
      "[FICTIF] Nadia Lectrice",
      expect.any(String),
    );
  });
  it("protects the last owner from accidental role changes or removal", async () => {
    openTeam();
    const select = await screen.findByRole("combobox", {
      name: "Rôle de [FICTIF] Nadia",
    });
    expect((select as HTMLSelectElement).disabled).toBe(true);
    expect(
      (
        screen.getByRole("button", {
          name: "Retirer l’accès de [FICTIF] Nadia",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
    expect(companyApi.updateMember).not.toHaveBeenCalled();
  });
});
