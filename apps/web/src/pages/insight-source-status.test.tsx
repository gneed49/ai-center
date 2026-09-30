// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter, Route, Routes } from "react-router";
import { api } from "@/api/client";
import type { InsightDetail } from "@/api/types";
import { InsightDetailPage } from "./insight-detail-page";
import { InsightsPage } from "./insights-page";
vi.mock("@/api/client", async (original) => ({
  ...(await original<typeof import("@/api/client")>()),
  api: {
    insight: vi.fn(),
    insights: vi.fn(),
    actOnInsight: vi.fn(),
    resolveInsight: vi.fn(),
  },
}));
vi.mock("sonner", () => ({ toast: { error: vi.fn(), success: vi.fn() } }));
const fixture: InsightDetail = {
  insight: {
    public_id: "fixture-insight",
    project_public_id: "fixture-project",
    project_name: "[FICTIF] Projet",
    project_graph_version: 4,
    insight_type: "context_gap",
    source_status: "unknown",
    status: "open",
    severity: "notice",
    confidence: 0.5,
    title: "[FICTIF] Contexte incomplet",
    explanation: "[FICTIF] Une observation ne permet pas de conclure",
    resolution_justification: null,
    detected_at: "2026-09-21T10:00:00Z",
    updated_at: "2026-09-21T10:00:00Z",
  },
  sources: [
    {
      source_role: "observed",
      object_kind: "publication_observation",
      object_public_id: "fixture-observation",
      knowledge_public_id: null,
      version_public_id: "fixture-observation",
      version_title: "[FICTIF] État distant",
      version_statement: "[FICTIF] Lecture partielle",
      source_project_public_id: "fixture-company-scope",
      source_status: "unknown",
    },
  ],
};
let client: QueryClient;
beforeEach(() => {
  vi.resetAllMocks();
  client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  vi.mocked(api.insight).mockResolvedValue(fixture);
  vi.mocked(api.insights).mockResolvedValue([fixture.insight]);
  vi.mocked(api.actOnInsight).mockResolvedValue(fixture);
});
afterEach(() => {
  cleanup();
  client.clear();
});
function openPage(list = false) {
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter
        initialEntries={[
          list
            ? "/insights"
            : "/projects/fixture-project/insights/fixture-insight",
        ]}
      >
        <Routes>
          <Route
            path="projects/:projectId/insights/:insightId"
            element={<InsightDetailPage />}
          />
          <Route path="insights" element={<InsightsPage />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}
describe("source validity separate from insight workflow", () => {
  it("labels insufficient context without calling it a contradiction and exposes exact source provenance", async () => {
    openPage();
    await screen.findByRole("heading", { name: "[FICTIF] Contexte incomplet" });
    expect(
      screen.getByText(/il ne prouve pas une contradiction/),
    ).toBeDefined();
    expect(screen.getAllByText("Actualité non vérifiée")).toHaveLength(2);
    expect(
      screen.getByText("publication_observation · fixture-observation"),
    ).toBeDefined();
    expect(screen.getByText("fixture-company-scope")).toBeDefined();
    fireEvent.change(
      screen.getByRole("textbox", { name: "Justification de la décision" }),
      { target: { value: "[FICTIF] Vérifier les sources" } },
    );
    expect(
      (
        screen.getByRole("button", {
          name: "Réviser pour résoudre",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
  });
  it("blocks stale acceptance and revision but still permits a justified dismissal", async () => {
    vi.mocked(api.insight).mockResolvedValue({
      ...fixture,
      insight: { ...fixture.insight, source_status: "stale" },
      sources: [
        {
          ...fixture.sources[0],
          object_kind: "knowledge_entry_version",
          knowledge_public_id: "fixture-knowledge",
          source_project_public_id: "fixture-project",
          source_status: "stale",
        },
      ],
    });
    openPage();
    await screen.findByText("Sources dépassées");
    fireEvent.change(
      screen.getByRole("textbox", { name: "Justification de la décision" }),
      { target: { value: "[FICTIF] Source remplacée" } },
    );
    expect(
      (
        screen.getByRole("button", {
          name: "Accepter le signal",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
    expect(
      (
        screen.getByRole("button", {
          name: "Réviser pour résoudre",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "Rejeter le signal" }));
    await waitFor(() =>
      expect(api.actOnInsight).toHaveBeenCalledWith(
        "fixture-project",
        "fixture-insight",
        "dismiss",
        "[FICTIF] Source remplacée",
        expect.any(String),
      ),
    );
    expect(api.resolveInsight).not.toHaveBeenCalled();
  });
  it("does not offer a mutation against a knowledge entry from a different project", async () => {
    vi.mocked(api.insight).mockResolvedValue({
      ...fixture,
      insight: { ...fixture.insight, source_status: "current" },
      sources: [
        {
          ...fixture.sources[0],
          knowledge_public_id: "fixture-foreign-knowledge",
          source_project_public_id: "fixture-other-project",
          source_status: "current",
        },
      ],
    });
    openPage();
    await screen.findByText("Sources actuelles");
    fireEvent.change(
      screen.getByRole("textbox", { name: "Justification de la décision" }),
      { target: { value: "[FICTIF] Lecture" } },
    );
    expect(
      (
        screen.getByRole("button", {
          name: "Réviser pour résoudre",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
  });
  it("keeps a missing source status unverified in the inbox", async () => {
    vi.mocked(api.insights).mockResolvedValue([
      { ...fixture.insight, source_status: null },
    ]);
    openPage(true);
    await screen.findByText("Contexte à compléter");
    expect(screen.getByText("Actualité non vérifiée")).toBeDefined();
    expect(screen.queryByText("Contradiction")).toBeNull();
  });
});
