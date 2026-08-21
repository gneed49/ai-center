export type UUID = string;

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
  context_pack: Record<string, unknown> | null;
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
  context_pack: Record<string, unknown>;
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

export interface InsightDetail {
  insight: InsightSummary;
  sources: Array<{
    source_role: string;
    object_kind: string;
    object_public_id: UUID;
    version_title: string | null;
    version_statement: string | null;
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
