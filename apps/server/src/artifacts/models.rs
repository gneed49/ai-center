use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

pub const ARTIFACT_TYPES: [&str; 5] = [
    "kickoff",
    "specification",
    "product_tickets",
    "technical_plan",
    "technical_tickets",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceInput {
    pub kind: String,
    /// For knowledge, this is the immutable knowledge VERSION's public ID.
    pub public_id: Uuid,
}

fn empty_object() -> Value {
    serde_json::json!({})
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateArtifact {
    pub artifact_type: String,
    pub title: String,
    #[serde(default)]
    pub body_markdown: String,
    #[serde(default = "empty_object")]
    pub structured_content: Value,
    #[serde(default)]
    pub sources: Vec<SourceInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveDraft {
    pub expected_version_id: Uuid,
    pub title: String,
    #[serde(default)]
    pub body_markdown: String,
    #[serde(default = "empty_object")]
    pub structured_content: Value,
    #[serde(default)]
    pub sources: Vec<SourceInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidateArtifact {
    pub expected_version_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ArtifactSummary {
    pub public_id: Uuid,
    pub project_id: Uuid,
    pub artifact_type: String,
    pub title: String,
    pub status: String,
    pub current_version_id: Uuid,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactVersion {
    pub public_id: Uuid,
    pub version: i32,
    pub title: String,
    pub body_markdown: String,
    pub structured_content: Value,
    pub sources: Vec<Value>,
    pub status: String,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub validated_at: Option<DateTime<Utc>>,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactDetail {
    pub artifact: ArtifactSummary,
    pub current_version: ArtifactVersion,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListArtifacts {
    pub q: Option<String>,
    #[serde(rename = "type")]
    pub artifact_type: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetDestination {
    pub artifact_type: String,
    pub provider: String,
    pub target_id: Option<String>,
    #[serde(default)]
    pub label: String,
    /// Zero when no setting exists in this scope. Required for concurrency.
    pub expected_revision: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResetDestination {
    pub artifact_type: String,
    pub expected_revision: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Destination {
    pub artifact_type: String,
    pub provider: String,
    pub target_id: Option<String>,
    pub label: String,
    pub origin: String,
    pub revision: i32,
    pub project_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Destinations {
    pub items: Vec<Destination>,
}
