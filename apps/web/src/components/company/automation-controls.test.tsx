// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ApiError } from "@/api/client";
import { companyControlsApi } from "@/api/company-controls";
import type { AutomationSettings } from "@/api/company-controls";
import { AutomationControls } from "./automation-controls";
vi.mock("@/api/company-controls", () => ({
  companyControlsApi: { automation: vi.fn(), setAutomation: vi.fn() },
}));
const settings: AutomationSettings = {
  enabled: true,
  generation: 4,
  limits: { calls_per_hour: 30, concurrent_calls: 2, call_timeout_seconds: 60 },
  calls_last_hour: 5,
  active_calls: 1,
  known_estimated_cost_usd: null,
  runs_without_cost: 5,
};
let client: QueryClient;
beforeEach(() => {
  vi.resetAllMocks();
  client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  vi.mocked(companyControlsApi.automation).mockResolvedValue(settings);
  vi.mocked(companyControlsApi.setAutomation).mockRejectedValue(
    new ApiError("Délai dépassé", 504),
  );
});
afterEach(() => {
  cleanup();
  client.clear();
});
function open(owner = true) {
  render(
    <QueryClientProvider client={client}>
      <AutomationControls owner={owner} />
    </QueryClientProvider>,
  );
}
describe("company automation controls", () => {
  it("keeps unknown cost unknown and explains pause limits", async () => {
    open();
    await screen.findByText("Non disponible");
    expect(screen.getByText("5 appel(s) sans estimation")).toBeDefined();
    expect(
      screen.getByText(/demandes déjà acceptées par un service externe/),
    ).toBeDefined();
    expect(companyControlsApi.setAutomation).not.toHaveBeenCalled();
  });
  it("uses the displayed generation and preserves a pending pause command after a concurrent refresh", async () => {
    open();
    fireEvent.click(
      await screen.findByRole("button", {
        name: "Mettre l’automatisation en pause",
      }),
    );
    await screen.findByText("Le changement n’a pas pu être confirmé");
    await act(async () => {
      client.setQueryData(["automation"], { ...settings, generation: 5 });
    });
    fireEvent.click(screen.getByRole("button", { name: "Réessayer" }));
    await waitFor(() =>
      expect(companyControlsApi.setAutomation).toHaveBeenCalledTimes(2),
    );
    const calls = vi.mocked(companyControlsApi.setAutomation).mock.calls;
    expect(calls[1]).toEqual(calls[0]);
    expect(calls[0][0]).toEqual({ enabled: false, expected_generation: 4 });
  });
  it("does not offer pause or resume to nonowners", async () => {
    open(false);
    await screen.findByText("Le propriétaire contrôle la pause et la reprise.");
    expect(
      screen.queryByRole("button", {
        name: "Mettre l’automatisation en pause",
      }),
    ).toBeNull();
  });
});
