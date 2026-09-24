// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import { request } from "@/api/client";
import { StewardProgress } from "./steward-progress";

vi.mock("@/api/client", () => ({ request: vi.fn() }));
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
function show(overrides = {}) {
  vi.mocked(request).mockResolvedValue({
    available_sources: 132,
    pending_sources: 5,
    examined_pairs: 20,
    omitted_neighbors: 9,
    max_sources: 10_000,
    max_neighbors_per_source: 12,
    max_provider_calls_per_hour: 6,
    progress: { status: "pending" },
    ...overrides,
  });
  render(
    <QueryClientProvider
      client={
        new QueryClient({ defaultOptions: { queries: { retry: false } } })
      }
    >
      <StewardProgress />
    </QueryClientProvider>,
  );
}
describe("company steward coverage", () => {
  it("shows remaining sources and omissions without claiming exhaustive coverage", async () => {
    show();
    await screen.findByText("5 source(s) à examiner");
    expect(screen.getByText(/9 voisin\(s\) omis/)).toBeTruthy();
    expect(
      screen.getByText(/ne garantit pas l’absence de contradictions/),
    ).toBeTruthy();
  });
  it("makes a catalog limit visible even before a worker updates its stored state", async () => {
    show({
      available_sources: 10_001,
      pending_sources: 10_001,
      progress: null,
    });
    await screen.findByText("L’analyse a atteint sa limite de sources");
    expect(
      screen.getByText(/l’opérateur doit revoir son périmètre/),
    ).toBeTruthy();
  });
});
