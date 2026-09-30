import { request } from "./client";
import { resolveApiUrl } from "./url";
import type { WorkspaceSummary } from "./types";
import type {
  CreateTeamInvitation,
  InvitationPreview,
  TeamInvitation,
  TeamMember,
} from "./team-types";

function command(input: unknown, key: string, method = "POST"): RequestInit {
  return {
    method,
    headers: { "Idempotency-Key": key },
    body: JSON.stringify(input),
  };
}
function secretRequest(input: {
  token: string;
  display_name?: string;
}): RequestInit {
  const base = resolveApiUrl({ configuredUrl: import.meta.env.VITE_API_URL });
  const url = new URL(base || "/", window.location.origin);
  if (
    url.protocol !== "https:" &&
    !(
      ["localhost", "127.0.0.1", "[::1]"].includes(url.hostname) &&
      ["localhost", "127.0.0.1", "[::1]"].includes(window.location.hostname)
    )
  )
    throw new Error(
      "Ouvrez cette invitation depuis l’adresse sécurisée de votre entreprise.",
    );
  return { method: "POST", cache: "no-store", body: JSON.stringify(input) };
}
export const teamApi = {
  members: () => request<TeamMember[]>("/api/team/members"),
  profile: (displayName: string, key: string) =>
    request<TeamMember>(
      "/api/team/profile",
      command({ display_name: displayName }, key, "PATCH"),
    ),
  invitations: () => request<TeamInvitation[]>("/api/team/invitations"),
  createInvitation: (input: CreateTeamInvitation, key: string) =>
    request<TeamInvitation>("/api/team/invitations", command(input, key)),
  revokeInvitation: (id: string, key: string) =>
    request<TeamInvitation>(
      `/api/team/invitations/${encodeURIComponent(id)}/revoke`,
      command({}, key),
    ),
  preview: async (id: string, token: string) =>
    request<InvitationPreview>(
      `/api/team/invitations/${encodeURIComponent(id)}/preview`,
      secretRequest({ token }),
    ),
  accept: async (id: string, token: string, displayName: string) =>
    request<WorkspaceSummary>(
      `/api/team/invitations/${encodeURIComponent(id)}/accept`,
      secretRequest({ token, display_name: displayName }),
    ),
};
