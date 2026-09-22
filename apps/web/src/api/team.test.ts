// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { teamApi } from "./team";
import { setRequestIdentity } from "./request-context";
beforeEach(() =>
  setRequestIdentity("fixture-actor", null, "fixture-access-token"),
);
afterEach(() => vi.unstubAllGlobals());
describe("invitation HTTP secrecy", () => {
  it("keeps the synthetic token only in preview/accept bodies and out of durable command headers", async () => {
    const fetch = vi
      .fn()
      .mockImplementation(() => Promise.resolve(Response.json({})));
    vi.stubGlobal("fetch", fetch);
    const syntheticToken = "b".repeat(64);
    await teamApi.preview("fixture-invitation", syntheticToken);
    await teamApi.accept(
      "fixture-invitation",
      syntheticToken,
      "[FICTIF] Nadia",
    );
    for (const [url, init] of fetch.mock.calls) {
      expect(url).not.toContain(syntheticToken);
      expect(url).not.toContain("?");
      expect(init.headers["Idempotency-Key"]).toBeUndefined();
      expect(init.cache).toBe("no-store");
      expect(JSON.parse(init.body).token).toBe(syntheticToken);
    }
    expect(fetch.mock.calls[1][1].headers.Authorization).toBe(
      "Bearer fixture-access-token",
    );
    expect(
      fetch.mock.calls[1][1].headers["X-AI-Center-Workspace-Id"],
    ).toBeUndefined();
  });
});
