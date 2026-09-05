use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Health {
    pub status: &'static str,
    pub service: &'static str,
    pub database: &'static str,
    pub agent_mode: &'static str,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateProject {
    pub name: String,
    #[serde(default)]
    pub objective: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ProjectSummary {
    pub public_id: Uuid,
    pub name: String,
    pub objective: String,
    pub summary: String,
    pub status: String,
    pub graph_version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct WorkspaceSummary {
    pub public_id: Uuid,
    pub name: String,
    pub role: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ContextNode {
    pub public_id: Uuid,
    pub node_key: String,
    pub title: String,
    pub description: String,
    pub summary: String,
    pub profile_key: String,
    pub profile_name: String,
    pub scope_kind: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct SessionSummary {
    pub public_id: Uuid,
    pub project_public_id: Uuid,
    pub title: String,
    pub status: String,
    pub node_key: String,
    pub scope_kind: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct KnowledgeSummary {
    pub public_id: Uuid,
    pub version_public_id: Uuid,
    pub version_number: i32,
    pub entry_type: String,
    pub title: String,
    pub statement: String,
    pub rationale: String,
    pub node_key: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct DeliverableSummary {
    pub public_id: Uuid,
    pub deliverable_type: String,
    pub title: String,
    pub summary: String,
    pub content: Value,
    pub status: String,
    pub coverage_status: String,
    pub version: i32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct InsightSummary {
    pub public_id: Uuid,
    pub project_public_id: Uuid,
    pub project_name: String,
    pub project_graph_version: i64,
    pub insight_type: String,
    pub status: String,
    pub severity: String,
    pub confidence: f64,
    pub title: String,
    pub explanation: String,
    pub resolution_justification: Option<String>,
    pub detected_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ProjectSnapshot {
    pub project: ProjectSummary,
    pub nodes: Vec<ContextNode>,
    pub sessions: Vec<SessionSummary>,
    pub knowledge: Vec<KnowledgeSummary>,
    pub deliverables: Vec<DeliverableSummary>,
    pub insights: Vec<InsightSummary>,
    pub gate: Option<GateResult>,
    pub latest_handoff: Option<HandoffView>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateSession {
    pub node_key: String,
    pub title: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessage {
    pub content: String,
    pub client_message_id: Uuid,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProposalDraft {
    pub entry_type: String,
    pub title: String,
    pub statement: String,
    #[serde(default)]
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTurn {
    pub response: String,
    pub proposals: Vec<ProposalDraft>,
    #[serde(default)]
    pub sources: Vec<Uuid>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct MessageView {
    pub public_id: Uuid,
    pub role: String,
    pub content: String,
    pub agent_scope: Option<String>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ProposalView {
    pub public_id: Uuid,
    pub entry_type: String,
    pub title: String,
    pub statement: String,
    pub rationale: String,
    pub status: String,
    pub source_data: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct SessionView {
    pub session: SessionSummary,
    pub messages: Vec<MessageView>,
    pub proposals: Vec<ProposalView>,
    pub context_pack: Option<ContextPackSummary>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DecideProposals {
    pub proposal_ids: Vec<Uuid>,
    pub decision: ProposalDecision,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalDecision {
    Confirm,
    Reject,
}

#[derive(Debug, Serialize)]
pub struct CommitResult {
    pub confirmed: Vec<Uuid>,
    pub rejected: Vec<Uuid>,
    pub graph_version: i64,
    pub insight_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub public_id: Option<Uuid>,
    pub status: String,
    pub graph_version: i64,
    pub missing: Vec<String>,
    pub warnings: Vec<String>,
    pub counts: GateCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateCounts {
    pub business_rules: i64,
    pub requirements: i64,
    pub acceptance_criteria: i64,
    pub open_questions: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InsightAction {
    pub action: InsightDecision,
    pub justification: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InsightDecision {
    Accept,
    Dismiss,
    Resolve,
}

#[derive(Debug, Serialize, FromRow)]
pub struct HistoryEvent {
    pub public_id: Uuid,
    pub action: String,
    pub object_kind: String,
    pub object_public_id: Uuid,
    pub before_state: Option<Value>,
    pub after_state: Option<Value>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateHandoff {
    pub source_session_id: Uuid,
    pub context_pack_id: Uuid,
}

#[derive(Debug, Clone, Serialize)]
pub struct HandoffView {
    pub public_id: Uuid,
    pub context_pack_public_id: Uuid,
    pub target_session_public_id: Uuid,
    pub status: String,
    pub context_pack: ContextPackSummary,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompileContextPack {
    pub source_session_id: Uuid,
    pub task_kind: String,
    #[serde(default)]
    pub token_budget: Option<i32>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ContextPackSelectionItemView {
    pub candidate_public_id: Uuid,
    pub decision: String,
    pub reason_code: String,
    pub explanation: String,
    pub rank: Option<i32>,
    pub estimated_tokens: i32,
    pub is_mandatory: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextPackSummary {
    pub public_id: Uuid,
    pub version: i32,
    pub status: String,
    pub source_graph_version: i64,
    pub compiler_version: String,
    pub selection_mode: String,
    pub content_hash: String,
    pub token_budget: i32,
    pub token_count: i32,
    pub compiled_at: DateTime<Utc>,
    pub invalidated_at: Option<DateTime<Utc>>,
    pub stale_reason: Option<String>,
    pub content: Value,
    pub selection_items: Vec<ContextPackSelectionItemView>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateTechnicalPlan {
    pub session_id: Uuid,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CoverageItem {
    pub requirement_public_id: Uuid,
    pub requirement_version_public_id: Uuid,
    pub requirement_title: String,
    pub requirement_statement: String,
    pub deliverable_public_id: Uuid,
    pub section_public_id: Option<Uuid>,
    pub evidence_public_id: Option<Uuid>,
    pub status: String,
    pub explanation: String,
}

#[derive(Debug, Serialize)]
pub struct CoverageView {
    pub total: usize,
    pub covered: usize,
    pub partial: usize,
    pub missing: usize,
    pub items: Vec<CoverageItem>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct InsightSourceView {
    pub source_role: String,
    pub object_kind: String,
    pub object_public_id: Uuid,
    pub knowledge_public_id: Option<Uuid>,
    pub version_public_id: Option<Uuid>,
    pub version_title: Option<String>,
    pub version_statement: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InsightDetail {
    pub insight: InsightSummary,
    pub sources: Vec<InsightSourceView>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReviseKnowledge {
    pub statement: String,
    pub title: Option<String>,
    pub rationale: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RevisionResult {
    pub knowledge_public_id: Uuid,
    pub version_public_id: Uuid,
    pub version_number: i32,
    pub graph_version: i64,
    pub insight_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResolveInsight {
    pub expected_graph_version: i64,
    pub justification: String,
    pub mutations: Vec<InsightResolutionMutation>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InsightResolutionMutation {
    ReviseKnowledge {
        knowledge_public_id: Uuid,
        expected_version_public_id: Uuid,
        statement: String,
        title: Option<String>,
        rationale: Option<String>,
    },
}
