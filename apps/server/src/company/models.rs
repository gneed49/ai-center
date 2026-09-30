//! Public contracts for the company workspace and its bounded graph projection.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::ProjectSummary;

#[derive(Debug, Serialize, FromRow)]
pub struct CompanyWorkspace {
    pub public_id: Uuid,
    pub name: String,
    pub description: String,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScopeRef {
    pub kind: String,
    pub project_public_id: Uuid,
}

#[derive(Debug, Serialize, FromRow)]
pub struct AgentSummary {
    pub node_public_id: Uuid,
    pub project_public_id: Uuid,
    pub scope_kind: String,
    pub node_key: String,
    pub profile_key: String,
    pub name: String,
    pub role: String,
    pub requires_context_pack: bool,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CompanyMember {
    pub public_id: Uuid,
    pub actor_id: Uuid,
    pub role: String,
    pub invitation_status: String,
    pub accepted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct CompanyOverview {
    pub workspace: CompanyWorkspace,
    pub company_scope: Option<ScopeRef>,
    pub agents: Vec<AgentSummary>,
    pub projects: Vec<ProjectSummary>,
    pub members: Vec<CompanyMember>,
    pub setup_complete: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompanyInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CreateCompany {
    pub public_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateMember {
    pub role: Option<String>,
    pub invitation_status: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct GraphQuery {
    pub project_id: Option<Uuid>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct GraphNode {
    pub id: Uuid,
    pub kind: String,
    pub project_public_id: Uuid,
    pub scope_kind: String,
    pub label: String,
    pub status: String,
    pub version_public_id: Option<Uuid>,
    pub version_number: Option<i32>,
    pub source_url: Option<String>,
    pub app_path: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct GraphEdge {
    pub id: Uuid,
    pub source_kind: String,
    pub source_public_id: Uuid,
    pub source_project_public_id: Uuid,
    pub target_kind: String,
    pub target_public_id: Uuid,
    pub target_project_public_id: Uuid,
    pub edge_type: String,
    pub status: String,
    pub provenance: serde_json::Value,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ScopeVersion {
    pub project_public_id: Uuid,
    pub graph_version: i64,
}

#[derive(Debug, Serialize)]
pub struct GraphLimits {
    pub max_nodes: u32,
    pub max_edges: u32,
}

#[derive(Debug, Serialize)]
pub struct GraphView {
    pub workspace_public_id: Uuid,
    pub project_public_id: Option<Uuid>,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub source_graph_versions: Vec<ScopeVersion>,
    pub truncated: bool,
    pub limits: GraphLimits,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct GraphEndpoint {
    pub kind: String,
    pub public_id: Uuid,
    pub project_public_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateGraphEdge {
    pub public_id: Uuid,
    pub source: GraphEndpoint,
    pub target: GraphEndpoint,
    pub edge_type: String,
}
