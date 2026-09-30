import { describe, expect, it } from "vitest";
import {
  safeWorkToolUrl,
  validExternalObjectId,
} from "./work-tool-publication";
describe("external publication navigation", () => {
  it("opens only verified provider HTTPS hosts", () => {
    expect(
      safeWorkToolUrl("github", "https://github.com/fictif/example/issues/42"),
    ).toBe("https://github.com/fictif/example/issues/42");
    for (const url of [
      "javascript:alert(1)",
      "https://github.com.evil.invalid/x",
      "https://user:password@github.com/x",
      "http://github.com/x",
      "https://github.com:444/x",
    ])
      expect(safeWorkToolUrl("github", url)).toBeNull();
    expect(safeWorkToolUrl("notion", "https://linear.app/a")).toBeNull();
  });
  it("accepts immutable identifiers and never arbitrary reconciliation URLs", () => {
    expect(validExternalObjectId("github", "42")).toBe(true);
    expect(
      validExternalObjectId("github", "https://github.com/org/repo/issues/42"),
    ).toBe(false);
    expect(
      validExternalObjectId("notion", "11111111-1111-4111-8111-111111111111"),
    ).toBe(true);
    expect(validExternalObjectId("linear", "https://linear.app/item")).toBe(
      false,
    );
  });
});
