import type {
  CompileContextPackInput,
  CommitResult,
  ContextPackSummary,
  CoverageView,
  DeliverableSummary,
  ExternalEvidence,
  ExternalReferenceSummary,
  ExternalReferenceView,
  GateResult,
  HandoffView,
  HistoryEvent,
  InsightDetail,
  InsightSummary,
  ProjectSnapshot,
  ProjectSummary,
  ResolveInsightInput,
  SessionView,
  UUID,
  WorkspaceSummary,
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
  let response: Response;
  try {
    response = await fetch(`${API_URL}${path}`, {
      ...init,
      headers: {
        "Content-Type": "application/json",
        ...requestContextHeaders(),
        ...init?.headers,
      },
    });
  } catch (error) {
    throw new ApiError(
      error instanceof Error
        ? error.message
        : "Le serveur est actuellement injoignable.",
      0,
      "network_error",
    );
  }
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
  if (response.status === 204) return undefined as T;
  return response.json() as Promise<T>;
}

function requestContextHeaders(): Record<string, string> {
  if (typeof window === "undefined") return {};
  const token = window.localStorage.getItem("ai-center.access-token");
  const workspaceId =
    window.localStorage.getItem("ai-center.workspace-id") ??
    import.meta.env.VITE_WORKSPACE_ID;
  return {
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
    ...(workspaceId ? { "X-AI-Center-Workspace-Id": workspaceId } : {}),
  };
}

export function createIdempotencyKey(): string {
  return crypto.randomUUID();
}

function mutationHeaders(idempotencyKey: string = createIdempotencyKey()) {
  return { "Idempotency-Key": idempotencyKey };
}

export interface HandoffIdempotencyKeys {
  compile: string;
  create: string;
}

export function createHandoffIdempotencyKeys(): HandoffIdempotencyKeys {
  return {
    compile: createIdempotencyKey(),
    create: createIdempotencyKey(),
  };
}

async function prepareHandoff(
  projectId: UUID,
  sourceSessionId: UUID,
  idempotencyKeys: HandoffIdempotencyKeys = createHandoffIdempotencyKeys(),
) {
  const contextPack = await request<ContextPackSummary>(
    `/api/projects/${projectId}/context-packs`,
    {
      method: "POST",
      headers: mutationHeaders(idempotencyKeys.compile),
      body: JSON.stringify({
        source_session_id: sourceSessionId,
        task_kind: "technical-delivery-plan",
        token_budget: 12_000,
      } satisfies CompileContextPackInput),
    },
  );
  return request<HandoffView>(`/api/projects/${projectId}/handoffs`, {
    method: "POST",
    headers: mutationHeaders(idempotencyKeys.create),
    body: JSON.stringify({
      source_session_id: sourceSessionId,
      context_pack_id: contextPack.public_id,
    }),
  });
}

export const api = {
  workspaces: () => request<WorkspaceSummary[]>("/api/workspaces"),
  projects: () => request<ProjectSummary[]>("/api/projects"),
  createProject: (
    input: { name: string; objective: string },
    idempotencyKey: string = createIdempotencyKey(),
  ) =>
    request<ProjectSummary>("/api/projects", {
      method: "POST",
      headers: mutationHeaders(idempotencyKey),
      body: JSON.stringify(input),
    }),
  snapshot: (projectId: UUID) =>
    request<ProjectSnapshot>(`/api/projects/${projectId}/snapshot`),
  createSession: (
    projectId: UUID,
    nodeKey: string,
    title?: string,
    idempotencyKey: string = createIdempotencyKey(),
  ) =>
    request<SessionView>(`/api/projects/${projectId}/sessions`, {
      method: "POST",
      headers: mutationHeaders(idempotencyKey),
      body: JSON.stringify({ node_key: nodeKey, title }),
    }),
  session: (projectId: UUID, sessionId: UUID) =>
    request<SessionView>(`/api/projects/${projectId}/sessions/${sessionId}`),
  sendMessage: (
    projectId: UUID,
    sessionId: UUID,
    content: string,
    idempotencyKey: string = createIdempotencyKey(),
  ) =>
    request<SessionView>(
      `/api/projects/${projectId}/sessions/${sessionId}/messages`,
      {
        method: "POST",
        headers: mutationHeaders(idempotencyKey),
        body: JSON.stringify({
          content,
          client_message_id: idempotencyKey,
        }),
      },
    ),
  decideProposals: (
    projectId: UUID,
    sessionId: UUID,
    proposalIds: UUID[],
    decision: "confirm" | "reject",
    idempotencyKey: string = createIdempotencyKey(),
  ) =>
    request<CommitResult>(
      `/api/projects/${projectId}/sessions/${sessionId}/proposals/decision`,
      {
        method: "POST",
        headers: mutationHeaders(idempotencyKey),
        body: JSON.stringify({ proposal_ids: proposalIds, decision }),
      },
    ),
  evaluateGate: (
    projectId: UUID,
    idempotencyKey: string = createIdempotencyKey(),
  ) =>
    request<GateResult>(
      `/api/projects/${projectId}/gates/product-ready/evaluate`,
      { method: "POST", headers: mutationHeaders(idempotencyKey) },
    ),
  featureBrief: (
    projectId: UUID,
    idempotencyKey: string = createIdempotencyKey(),
  ) =>
    request<DeliverableSummary>(
      `/api/projects/${projectId}/deliverables/feature-brief`,
      { method: "POST", headers: mutationHeaders(idempotencyKey) },
    ),
  compileContextPack: (
    projectId: UUID,
    input: CompileContextPackInput,
    idempotencyKey: string = createIdempotencyKey(),
  ) =>
    request<ContextPackSummary>(`/api/projects/${projectId}/context-packs`, {
      method: "POST",
      headers: mutationHeaders(idempotencyKey),
      body: JSON.stringify(input),
    }),
  handoff: (
    projectId: UUID,
    sourceSessionId: UUID,
    contextPackId: UUID,
    idempotencyKey: string = createIdempotencyKey(),
  ) =>
    request<HandoffView>(`/api/projects/${projectId}/handoffs`, {
      method: "POST",
      headers: mutationHeaders(idempotencyKey),
      body: JSON.stringify({
        source_session_id: sourceSessionId,
        context_pack_id: contextPackId,
      }),
    }),
  prepareHandoff,
  latestHandoff: (projectId: UUID) =>
    request<HandoffView | null>(`/api/projects/${projectId}/handoffs/latest`),
  technicalPlan: (
    projectId: UUID,
    sessionId: UUID,
    idempotencyKey: string = createIdempotencyKey(),
  ) =>
    request<DeliverableSummary>(
      `/api/projects/${projectId}/deliverables/technical-plan`,
      {
        method: "POST",
        headers: mutationHeaders(idempotencyKey),
        body: JSON.stringify({ session_id: sessionId }),
      },
    ),
  coverage: (projectId: UUID) =>
    request<CoverageView>(`/api/projects/${projectId}/coverage`),
  insights: () => request<InsightSummary[]>("/api/insights"),
  insight: (projectId: UUID, insightId: UUID) =>
    request<InsightDetail>(`/api/projects/${projectId}/insights/${insightId}`),
  actOnInsight: (
    projectId: UUID,
    insightId: UUID,
    action: "accept" | "dismiss",
    justification: string,
    idempotencyKey: string = createIdempotencyKey(),
  ) =>
    request<InsightDetail>(`/api/projects/${projectId}/insights/${insightId}`, {
      method: "PATCH",
      headers: mutationHeaders(idempotencyKey),
      body: JSON.stringify({ action, justification }),
    }),
  resolveInsight: (
    projectId: UUID,
    insightId: UUID,
    input: ResolveInsightInput,
    idempotencyKey: string = createIdempotencyKey(),
  ) =>
    request<InsightDetail>(
      `/api/projects/${projectId}/insights/${insightId}/resolve`,
      {
        method: "POST",
        headers: mutationHeaders(idempotencyKey),
        body: JSON.stringify(input),
      },
    ),
  history: (projectId: UUID) =>
    request<HistoryEvent[]>(`/api/projects/${projectId}/history`),
  externalReferences: (projectId: UUID) =>
    request<ExternalReferenceSummary[]>(
      `/api/projects/${projectId}/external-references`,
    ),
  createExternalReference: (
    projectId: UUID,
    url: string,
    idempotencyKey: string = createIdempotencyKey(),
    contextPackId?: UUID,
  ) =>
    request<ExternalReferenceView>(
      `/api/projects/${projectId}/external-references`,
      {
        method: "POST",
        headers: mutationHeaders(idempotencyKey),
        body: JSON.stringify({
          url,
          ...(contextPackId
            ? {
                tracking: {
                  context_pack_id: contextPackId,
                  transmission_confirmed: true,
                },
              }
            : {}),
        }),
      },
    ),
  externalReference: (referenceId: UUID) =>
    request<ExternalReferenceView>(`/api/external-references/${referenceId}`),
  refreshExternalReference: (
    referenceId: UUID,
    idempotencyKey: string = createIdempotencyKey(),
  ) =>
    request<ExternalReferenceView>(
      `/api/external-references/${referenceId}/refresh`,
      { method: "POST", headers: mutationHeaders(idempotencyKey) },
    ),
  createExternalEvidence: (
    referenceId: UUID,
    input: {
      artifact_id?: UUID;
      requirement_id: UUID;
      deliverable_id: UUID;
      deliverable_section_id: UUID;
      title: string;
      description?: string;
    },
    idempotencyKey: string = createIdempotencyKey(),
  ) =>
    request<ExternalEvidence>(
      `/api/external-references/${referenceId}/evidence`,
      {
        method: "POST",
        headers: mutationHeaders(idempotencyKey),
        body: JSON.stringify(input),
      },
    ),
  reviewExternalEvidence: (
    referenceId: UUID,
    evidenceId: UUID,
    decision: "validate" | "reject",
    idempotencyKey: string = createIdempotencyKey(),
  ) =>
    request<ExternalEvidence>(
      `/api/external-references/${referenceId}/evidence/${evidenceId}/review`,
      {
        method: "POST",
        headers: mutationHeaders(idempotencyKey),
        body: JSON.stringify({ decision }),
      },
    ),
  reviseKnowledge: (
    projectId: UUID,
    knowledgeId: UUID,
    statement: string,
    rationale?: string,
    idempotencyKey: string = createIdempotencyKey(),
  ) =>
    request(`/api/projects/${projectId}/knowledge/${knowledgeId}`, {
      method: "PATCH",
      headers: mutationHeaders(idempotencyKey),
      body: JSON.stringify({ statement, rationale }),
    }),
};
