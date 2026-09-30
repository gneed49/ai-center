import { describe, it, expect } from "vitest";
import { codeReadValidation, githubCodeUrl } from "./code-evidence";
const commit = "a".repeat(40);
describe("bounded GitHub source references", () => {
  it("requires an immutable complete commit and explicit bounded relative paths", () => {
    expect(
      codeReadValidation("fiction/example", commit, ["src/app.ts"]),
    ).toBeNull();
    expect(
      codeReadValidation("fiction/example", "main", ["src/app.ts"]),
    ).not.toBeNull();
    for (const paths of [
      [],
      ["../key"],
      ["/root"],
      ["src/../key"],
      ["src\\key"],
      ["a", "a"],
      ["a/b/c/d/e/f/g/h/i"],
      Array.from({ length: 11 }, (_, i) => `${i}.ts`),
    ]) {
      expect(
        codeReadValidation("fiction/example", commit, paths),
      ).not.toBeNull();
    }
  });
  it("builds a fixed GitHub URL for exact encoded file and lines, never a supplied host", () => {
    expect(
      githubCodeUrl("fiction/example", commit, "src/my file#1.ts", 2, 3),
    ).toBe(
      `https://github.com/fiction/example/blob/${commit}/src/my%20file%231.ts#L2-L3`,
    );
    expect(githubCodeUrl("https://evil.test", commit, "a.ts", 1, 2)).toBeNull();
    expect(githubCodeUrl("fiction/example", "main", "a.ts")).toBeNull();
  });
});
