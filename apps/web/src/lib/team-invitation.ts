import type { CreateTeamInvitation, InvitationRole } from "@/api/team-types";

function hex(bytes: Uint8Array) {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join(
    "",
  );
}
export async function prepareTeamInvitation(input: {
  role: InvitationRole;
  label: string;
  expires_in_days: number;
}) {
  if (!crypto.subtle)
    throw new Error(
      "La création d’une invitation exige une connexion sécurisée.",
    );
  const token = hex(crypto.getRandomValues(new Uint8Array(32)));
  const hash = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(token),
  );
  const payload: CreateTeamInvitation = {
    ...input,
    public_id: crypto.randomUUID(),
    token_hash: hex(new Uint8Array(hash)),
  };
  return { key: crypto.randomUUID(), token, payload };
}
export function invitationLink(origin: string, id: string, token: string) {
  const url = new URL(`/join/${encodeURIComponent(id)}`, origin);
  url.hash = new URLSearchParams({ token }).toString();
  return url.toString();
}
export function invitationToken(hash: string) {
  const token = new URLSearchParams(hash.replace(/^#/, "")).get("token");
  return token && /^[a-f0-9]{64}$/.test(token) ? token : null;
}
