import { describe, expect, it } from "vitest";
import {
  invitationLink,
  invitationToken,
  prepareTeamInvitation,
} from "./team-invitation";

describe("team invitation secret boundary", () => {
  it("generates independent 256-bit tokens but only sends their SHA-256 digest", async () => {
    const input = {
      role: "viewer" as const,
      label: "[FICTIF] Relecture",
      expires_in_days: 3,
    };
    const first = await prepareTeamInvitation(input);
    const second = await prepareTeamInvitation(input);
    expect(first.token).toMatch(/^[a-f0-9]{64}$/);
    expect(second.token).not.toBe(first.token);
    const digest = await crypto.subtle.digest(
      "SHA-256",
      new TextEncoder().encode(first.token),
    );
    const hash = Array.from(new Uint8Array(digest), (byte) =>
      byte.toString(16).padStart(2, "0"),
    ).join("");
    expect(first.payload.token_hash).toBe(hash);
    expect(JSON.stringify(first.payload)).not.toContain(first.token);
    expect(first.payload.public_id).not.toBe(second.payload.public_id);
    expect(first.key).not.toBe(second.key);
  });
  it("carries the synthetic token only in the fragment and rejects incomplete links", () => {
    const syntheticToken = "a".repeat(64);
    const url = new URL(
      invitationLink(
        "https://fixture.invalid",
        "fixture-invitation",
        syntheticToken,
      ),
    );
    expect(url.pathname).toBe("/join/fixture-invitation");
    expect(url.search).toBe("");
    expect(invitationToken(url.hash)).toBe(syntheticToken);
    expect(invitationToken("#token=too-short")).toBeNull();
    expect(invitationToken("#else=fixture")).toBeNull();
  });
});
