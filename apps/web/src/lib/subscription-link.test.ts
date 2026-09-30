import { beforeEach, describe, expect, it, vi } from "vitest";
import { openUrl } from "@tauri-apps/plugin-opener";
import { openSubscriptionLink } from "./subscription-link";

vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
beforeEach(() => vi.resetAllMocks());
describe("system browser bridge with a mocked opener", () => {
  it("opens only the validated official URL", async () => {
    const url = "https://claude.com/cai/oauth/authorize?state=fixture";
    await openSubscriptionLink(url);
    expect(openUrl).toHaveBeenCalledWith(url);
  });
  it("rejects an arbitrary URL before calling the native bridge", async () => {
    await expect(
      openSubscriptionLink("https://evil.example/authorize"),
    ).rejects.toThrow();
    expect(openUrl).not.toHaveBeenCalled();
  });
  it("never propagates a rejected native error containing an auth URL", async () => {
    const url = "https://claude.com/cai/oauth/authorize?state=fixture-private";
    vi.mocked(openUrl).mockRejectedValue(new Error(`ForbiddenUrl ${url}`));
    await expect(openSubscriptionLink(url)).rejects.toThrow(
      "Impossible d’ouvrir le navigateur pour cette connexion.",
    );
  });
});
