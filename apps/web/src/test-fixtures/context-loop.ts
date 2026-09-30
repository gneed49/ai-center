import type {
  ContextPackSummary,
  ProjectSnapshot,
  SessionView,
} from "@/api/types";

/** Synthetic fixtures only; no real workspace, project or provider data. */
export const fixturePack: ContextPackSummary = {
  public_id: "30000000-0000-4000-8000-000000000001",
  version: 2,
  status: "current",
  source_graph_version: 4,
  compiler_version: "fixture-compiler/1",
  selection_mode: "deterministic",
  content_hash: "fixture-hash",
  token_budget: 12000,
  token_count: 100,
  compiled_at: "2026-09-21T10:00:00Z",
  invalidated_at: null,
  stale_reason: null,
  content: {
    objective: "[FICTIF] Vérifier un export de contexte",
    provenance: [
      {
        knowledge_public_id: "fixture-knowledge",
        version_public_id: "fixture-version",
        reason_code: "mandatory",
        explanation: "[FICTIF] Exigence confirmée",
      },
    ],
  },
  selection_items: [],
};

export const fixtureSession: SessionView = {
  session: {
    public_id: "fixture-session",
    title: "[FICTIF] Session Produit",
    status: "active",
    node_key: "product",
    scope_kind: "product",
    created_at: "2026-09-21T10:00:00Z",
    updated_at: "2026-09-21T10:00:00Z",
  },
  messages: [],
  proposals: [],
  context_pack: null,
};

export const fixtureSnapshot: ProjectSnapshot = {
  project: {
    public_id: "fixture-project",
    name: "[FICTIF] Projet de validation",
    objective: "[FICTIF] Éviter les doublons",
    summary: "",
    status: "active",
    graph_version: 4,
    created_at: "2026-09-21T10:00:00Z",
    updated_at: "2026-09-21T10:00:00Z",
  },
  nodes: [],
  sessions: [fixtureSession.session],
  knowledge: [],
  deliverables: [],
  insights: [],
  gate: null,
  latest_handoff: null,
};
