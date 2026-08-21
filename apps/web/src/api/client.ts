import type {
  CommitResult,
  CoverageView,
  DeliverableSummary,
  GateResult,
  HandoffView,
  HistoryEvent,
  InsightDetail,
  InsightSummary,
  ProjectSnapshot,
  ProjectSummary,
  SessionView,
  UUID,
} from "./types";
import { resolveApiUrl } from "./url";

const API_URL = resolveApiUrl({
  configuredUrl: import.meta.env.VITE_API_URL,
});

export class ApiError extends Error {
  readonly status: number;
  readonly code?: string;

  constructor(message: string, status: number, code?: string) {
    super(message);
    this.status = status;
    this.code = code;
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_URL}${path}`, {
    ...init,
    headers: { "Content-Type": "application/json", ...init?.headers },
  });
  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as {
      message?: string;
      code?: string;
    } | null;
    throw new ApiError(
      body?.message ?? `Erreur HTTP ${response.status}`,
      response.status,
      body?.code,
    );
  }
  return response.json() as Promise<T>;
}

export const api = {
  projects: () => request<ProjectSummary[]>("/api/projects"),
  createProject: (input: { name: string; objective: string }) =>
    request<ProjectSummary>("/api/projects", {
      method: "POST",
      body: JSON.stringify(input),
    }),
  snapshot: (projectId: UUID) =>
    request<ProjectSnapshot>(`/api/projects/${projectId}/snapshot`),
  createSession: (projectId: UUID, nodeKey: string, title?: string) =>
    request<SessionView>(`/api/projects/${projectId}/sessions`, {
      method: "POST",
      body: JSON.stringify({ node_key: nodeKey, title }),
    }),
  session: (sessionId: UUID) =>
    request<SessionView>(`/api/sessions/${sessionId}`),
  sendMessage: (sessionId: UUID, content: string) =>
    request<SessionView>(`/api/sessions/${sessionId}/messages`, {
      method: "POST",
      body: JSON.stringify({ content }),
    }),
  decideProposals: (
    sessionId: UUID,
    proposalIds: UUID[],
    decision: "confirm" | "reject",
  ) =>
    request<CommitResult>(`/api/sessions/${sessionId}/proposals/decision`, {
      method: "POST",
      body: JSON.stringify({ proposal_ids: proposalIds, decision }),
    }),
  evaluateGate: (projectId: UUID) =>
    request<GateResult>(
      `/api/projects/${projectId}/gates/product-ready/evaluate`,
      { method: "POST" },
    ),
  featureBrief: (projectId: UUID) =>
    request<DeliverableSummary>(
      `/api/projects/${projectId}/deliverables/feature-brief`,
      { method: "POST" },
    ),
  handoff: (projectId: UUID, sourceSessionId: UUID) =>
    request<HandoffView>(`/api/projects/${projectId}/handoffs`, {
      method: "POST",
      body: JSON.stringify({ source_session_id: sourceSessionId }),
    }),
  technicalPlan: (projectId: UUID, sessionId: UUID) =>
    request<DeliverableSummary>(
      `/api/projects/${projectId}/deliverables/technical-plan`,
      {
        method: "POST",
        body: JSON.stringify({ session_id: sessionId }),
      },
    ),
  coverage: (projectId: UUID) =>
    request<CoverageView>(`/api/projects/${projectId}/coverage`),
  insights: () => request<InsightSummary[]>("/api/insights"),
  insight: (insightId: UUID) =>
    request<InsightDetail>(`/api/insights/${insightId}`),
  actOnInsight: (
    insightId: UUID,
    action: "accept" | "dismiss" | "resolve",
    justification: string,
  ) =>
    request<InsightDetail>(`/api/insights/${insightId}`, {
      method: "PATCH",
      body: JSON.stringify({ action, justification }),
    }),
  history: (projectId: UUID) =>
    request<HistoryEvent[]>(`/api/projects/${projectId}/history`),
  reviseKnowledge: (knowledgeId: UUID, statement: string, rationale?: string) =>
    request(`/api/knowledge/${knowledgeId}`, {
      method: "PATCH",
      body: JSON.stringify({ statement, rationale }),
    }),
};
