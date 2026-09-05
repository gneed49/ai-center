export type UUID = string;

export interface WorkspaceSummary {
  public_id: UUID;
  name: string;
  role: "owner" | "editor" | "viewer";
}

export interface ProjectSummary {
  public_id: UUID;
  name: string;
  objective: string;
  summary: string;
  status: string;
  graph_version: number;
  created_at: string;
  updated_at: string;
}

export interface ContextNode {
  public_id: UUID;
  node_key: "product" | "tech";
  title: string;
  description: string;
  summary: string;
  profile_key: string;
  profile_name: string;
  scope_kind: string;
}

export interface SessionSummary {
  public_id: UUID;
  title: string;
  status: string;
  node_key: "product" | "tech";
  scope_kind: string;
  created_at: string;
  updated_at: string;
}

export interface KnowledgeSummary {
  public_id: UUID;
  version_public_id: UUID;
  version_number: number;
  entry_type: string;
  title: string;
  statement: string;
  rationale: string;
  node_key: string;
  created_at: string;
}

export interface DeliverableSummary {
  public_id: UUID;
  deliverable_type: string;
  title: string;
  summary: string;
  content: Record<string, unknown>;
  status: string;
  coverage_status: string;
  version: number;
  updated_at: string;
}

export interface InsightSummary {
  public_id: UUID;
  project_public_id: UUID;
  project_name: string;
  project_graph_version: number;
  insight_type: "contradiction" | "coverage_gap";
  status: string;
  severity: "notice" | "warning" | "blocking";
  confidence: number;
  title: string;
  explanation: string;
  resolution_justification: string | null;
  detected_at: string;
  updated_at: string;
}

export interface GateResult {
  public_id: UUID | null;
  status: "pending" | "passed" | "passed_with_warning" | "blocked";
  graph_version: number;
  missing: string[];
  warnings: string[];
  counts: {
    business_rules: number;
    requirements: number;
    acceptance_criteria: number;
    open_questions: number;
  };
}

export interface ProjectSnapshot {
  project: ProjectSummary;
  nodes: ContextNode[];
  sessions: SessionSummary[];
  knowledge: KnowledgeSummary[];
  deliverables: DeliverableSummary[];
  insights: InsightSummary[];
  gate: GateResult | null;
  latest_handoff: HandoffView | null;
}

export interface MessageView {
  public_id: UUID;
  role: "user" | "assistant" | "system";
  content: string;
  agent_scope: string | null;
  metadata: { sources?: UUID[] };
  created_at: string;
}

export interface ProposalView {
  public_id: UUID;
  entry_type: string;
  title: string;
  statement: string;
  rationale: string;
  status: "proposed" | "confirmed" | "rejected";
  source_data: { source_version_ids?: UUID[] };
  created_at: string;
}

export interface SessionView {
  session: SessionSummary;
  messages: MessageView[];
  proposals: ProposalView[];
  context_pack: ContextPackSummary | null;
}

export interface CommitResult {
  confirmed: UUID[];
  rejected: UUID[];
  graph_version: number;
  insight_ids: UUID[];
}

export interface HandoffView {
  public_id: UUID;
  context_pack_public_id: UUID;
  target_session_public_id: UUID;
  status: string;
  context_pack: ContextPackSummary;
  completed_at?: string | null;
}

export type ContextPackStatus = "current" | "stale" | "superseded";

export type ContextPackSelectionDecision = "included" | "excluded";

export interface ContextPackSelectionItem {
  candidate_public_id: UUID;
  decision: ContextPackSelectionDecision;
  reason_code: string;
  explanation: string;
  rank: number | null;
  estimated_tokens: number;
  is_mandatory: boolean;
}

export interface ContextPackProvenance {
  knowledge_public_id: UUID;
  version_public_id: UUID;
  reason_code: string;
  explanation: string;
}

export interface ContextPackKnowledge {
  knowledge_public_id: UUID;
  version_public_id: UUID;
  version_number: number;
  entry_type: string;
  title: string;
  statement: string;
  rationale: string;
  node_key: string;
}

export interface ContextPackContent {
  objective?: string;
  project_summary?: string;
  decisions?: KnowledgeSummary[];
  requirements?: KnowledgeSummary[];
  acceptance_criteria?: KnowledgeSummary[];
  constraints?: KnowledgeSummary[];
  open_questions?: KnowledgeSummary[];
  knowledge?: ContextPackKnowledge[];
  contract?: Record<string, unknown>;
  provenance?: ContextPackProvenance[];
  [key: string]: unknown;
}

export interface ContextPackSummary {
  public_id: UUID;
  version: number;
  status: ContextPackStatus;
  source_graph_version: number;
  compiler_version: string;
  selection_mode: "deterministic" | "hybrid";
  content_hash: string;
  token_budget: number;
  token_count: number;
  compiled_at: string;
  invalidated_at: string | null;
  stale_reason: string | null;
  content: ContextPackContent;
  selection_items: ContextPackSelectionItem[];
}

export interface CompileContextPackInput {
  source_session_id: UUID;
  task_kind: "technical-delivery-plan";
  token_budget?: number;
}

export interface CoverageItem {
  requirement_public_id: UUID;
  requirement_version_public_id: UUID;
  requirement_title: string;
  requirement_statement: string;
  deliverable_public_id: UUID;
  section_public_id: UUID | null;
  evidence_public_id: UUID | null;
  status: "covered" | "partial" | "missing";
  explanation: string;
}

export interface CoverageView {
  total: number;
  covered: number;
  partial: number;
  missing: number;
  items: CoverageItem[];
}

export type ExternalReferenceSyncStatus =
  "pending" | "current" | "stale" | "unavailable" | "error";

export interface ExternalReferenceObservation {
  public_id: UUID;
  observation_status: "current" | "stale" | "unavailable";
  content_hash: string;
  etag?: string;
  observed_state: {
    head_sha?: string;
    base_sha?: string;
    state?: string;
    checks?: Array<{
      kind?: "check_run" | "commit_status";
      name: string;
      status: string;
      conclusion?: string | null;
    }>;
    [key: string]: unknown;
  };
  provider_updated_at?: string;
  observed_at: string;
}

export interface ExternalEvidence {
  artifact_public_id?: UUID;
  public_id: UUID;
  requirement_public_id: UUID;
  requirement_version_public_id: UUID;
  deliverable_public_id: UUID;
  deliverable_section_public_id: UUID;
  evidence_type: string;
  title: string;
  description: string;
  source_reference: string;
  status: "candidate" | "valid" | "stale" | "unavailable" | "rejected";
  created_at: string;
}

export interface ExternalReferenceSummary {
  public_id: UUID;
  project_public_id: UUID;
  tool_connection_public_id: UUID;
  provider: "github";
  object_kind: "repository" | "pull_request" | "commit";
  external_id: string;
  canonical_url: string;
  repository_full_name: string;
  display_title: string;
  sync_status: ExternalReferenceSyncStatus;
  etag?: string;
  last_synced_at?: string;
  last_error_code?: string;
  created_at: string;
  updated_at: string;
}

export interface ExternalTracking {
  task_public_id: UUID;
  execution_public_id: UUID;
  context_pack_public_id: UUID;
  context_pack_version: number;
  context_pack_hash: string;
  context_pack_current: boolean;
  status: string;
  observed_result: Record<string, unknown>;
  artifacts: Array<{
    public_id: UUID;
    reference: string;
    metadata: {
      head_sha?: string;
      sync_status?: string;
      [key: string]: unknown;
    };
    created_at: string;
  }>;
  events: Array<{
    public_id: UUID;
    event_type: string;
    sequence_number: number;
    payload: Record<string, unknown>;
    created_at: string;
  }>;
}

export interface ExternalReferenceView extends ExternalReferenceSummary {
  tracking?: ExternalTracking[];
  latest_observation?: ExternalReferenceObservation;
  evidences: ExternalEvidence[];
}

export interface InsightDetail {
  insight: InsightSummary;
  sources: Array<{
    source_role: string;
    object_kind: string;
    object_public_id: UUID;
    knowledge_public_id: UUID | null;
    version_public_id: UUID | null;
    version_title: string | null;
    version_statement: string | null;
  }>;
}

export interface ResolveInsightInput {
  expected_graph_version: number;
  justification: string;
  mutations: Array<{
    kind: "revise_knowledge";
    knowledge_public_id: UUID;
    expected_version_public_id: UUID;
    statement: string;
    title?: string;
    rationale?: string;
  }>;
}

export interface HistoryEvent {
  public_id: UUID;
  action: string;
  object_kind: string;
  object_public_id: UUID;
  before_state: Record<string, unknown> | null;
  after_state: Record<string, unknown> | null;
  occurred_at: string;
}
