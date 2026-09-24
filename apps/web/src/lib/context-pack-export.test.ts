// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { fixturePack } from "@/test-fixtures/context-loop";
import { downloadContextPack } from "./context-pack-export";

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

describe("context export file download", () => {
  it("names the file by immutable pack and version and releases its temporary URL", () => {
    vi.useFakeTimers();
    const createObjectURL = vi.fn().mockReturnValue("blob:fixture-export");
    const revokeObjectURL = vi.fn();
    vi.stubGlobal("URL", { createObjectURL, revokeObjectURL });
    let filename = "";
    let href = "";
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(
      function (this: HTMLAnchorElement) {
        filename = this.download;
        href = this.href;
      },
    );
    downloadContextPack(fixturePack, "markdown", "# [FICTIF] canonical pack");
    expect(filename).toBe(`context-pack-${fixturePack.public_id}-v2.md`);
    expect(href).toBe("blob:fixture-export");
    expect(createObjectURL.mock.calls[0][0].type).toBe(
      "text/markdown;charset=utf-8",
    );
    expect(document.querySelector("a[download]")).toBeNull();
    vi.runAllTimers();
    expect(revokeObjectURL).toHaveBeenCalledWith("blob:fixture-export");
  });
});
