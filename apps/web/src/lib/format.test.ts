import { describe, expect, it } from "vitest";

import { humanize, shortId } from "./format";

describe("format helpers", () => {
  it("humanizes domain identifiers without losing their words", () => {
    expect(humanize("knowledge.committed")).toBe("knowledge · committed");
    expect(humanize("acceptance_criterion")).toBe("acceptance criterion");
  });

  it("renders stable compact provenance identifiers", () => {
    expect(shortId("dadd443b-f51a-480f-865b-b1ef8260966a")).toBe("dadd443b");
  });
});
