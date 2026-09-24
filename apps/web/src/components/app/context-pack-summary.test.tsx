// @vitest-environment jsdom
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { fixturePack } from "@/test-fixtures/context-loop";
import { ContextPackSummaryCard } from "./context-pack-summary";
afterEach(cleanup);
describe("authoritative context pack source freshness", () => {
  it("keeps a pack current after an unrelated graph change", () => {
    render(<ContextPackSummaryCard pack={fixturePack} graphVersion={5} />);
    expect(screen.getByText("Courant")).toBeDefined();
    expect(screen.queryByRole("alert")).toBeNull();
  });
  it("blocks server-stale sources even when the graph counter is unchanged", () => {
    render(
      <ContextPackSummaryCard
        pack={{
          ...fixturePack,
          status: "stale",
          stale_reason: "Source de la société révisée",
        }}
        graphVersion={4}
      />,
    );
    expect(screen.getByText("Obsolète")).toBeDefined();
    expect(screen.getByRole("alert").textContent).toContain(
      "Source de la société révisée",
    );
  });
});
