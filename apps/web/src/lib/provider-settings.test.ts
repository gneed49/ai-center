import { describe, expect, it } from "vitest";
import { ApiError } from "@/api/client";
import {
  officialLoginUrl,
  providerError,
  retryIdentity,
} from "./provider-settings";

describe("personal connection helpers", () => {
  it("reuses a retry identity only for the exact same request, without retaining the secret", async () => {
    const input = {
      id: "connection",
      api_key: "fixture-private-value",
      model: "model-a",
    };
    const first = await retryIdentity(input);
    expect(await retryIdentity(input, first)).toEqual(first);
    expect(
      (await retryIdentity({ ...input, api_key: "replacement-fixture" }, first))
        .key,
    ).not.toBe(first.key);
    expect(
      (await retryIdentity({ ...input, model: "model-b" }, first)).key,
    ).not.toBe(first.key);
    expect(JSON.stringify(first)).not.toContain(input.api_key);
  });
  it.each([
    "https://evil.example/oauth/authorize",
    "https://claude.com.evil.example/oauth/authorize",
    "https://attacker@claude.com/oauth/authorize",
    "javascript:alert(1)",
    "http://claude.com/oauth/authorize",
    "https://claude.com:444/oauth/authorize",
    "not-a-url",
  ])("rejects an unofficial login URL: %s", (url) =>
    expect(officialLoginUrl(url)).toBeUndefined(),
  );
  it("allows the current official Claude authorization destination", () => {
    const url =
      "https://claude.com/cai/oauth/authorize?code=true&state=fixture";
    expect(officialLoginUrl(url)).toBe(url);
  });
  it.each([
    new Error("fixture-secret"),
    new ApiError("fixture-secret", 502),
    new ApiError("fixture-secret", 401),
  ])("does not echo error content", (error) => {
    expect(providerError(error)).not.toContain("fixture-secret");
  });
});
