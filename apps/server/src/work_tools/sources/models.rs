use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_TEXT_BYTES: usize = 65_536;
pub const MAX_SNAPSHOT_BYTES: usize = 131_072;
pub const PROJECTION_VERSION: &str = "existing-tool-text-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolProvider {
    Linear,
    Notion,
}
impl ToolProvider {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::Notion => "notion",
        }
    }
    #[must_use]
    pub const fn object_kind(self) -> &'static str {
        match self {
            Self::Linear => "issue",
            Self::Notion => "page",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceCoverage {
    Complete,
    Partial,
    None,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OmissionReason {
    CommentsNotRead,
    AttachmentsNotRead,
    RelatedObjectsNotRead,
    PropertiesNotRead,
    EmbeddedContentNotRead,
    TranscriptsNotRead,
    UnknownBlocks,
    ProviderTruncated,
    LocalTextLimit,
    RemoteDateUnavailable,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinearState {
    pub id: Uuid,
    pub name: String,
    pub r#type: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SourceMetadata {
    Linear {
        identifier: String,
        team_id: Uuid,
        state: Option<LinearState>,
    },
    Notion {
        parent_type: String,
        parent_id: Option<Uuid>,
    },
}

/// Observed issue facts only. Never forward arbitrary provider metadata as context.
pub(crate) fn context_metadata(
    provider: ToolProvider,
    metadata: &serde_json::Value,
) -> serde_json::Value {
    use serde_json::{Value, json};

    if provider != ToolProvider::Linear {
        return json!({});
    }
    let Some(identifier) = metadata.get("identifier").and_then(Value::as_str) else {
        return json!({});
    };
    if super::locator::linear_identifier(identifier).as_deref() != Some(identifier) {
        return json!({});
    }
    let state = match metadata.get("state") {
        Some(Value::Null) => None,
        Some(value) => {
            let Ok(state) = serde_json::from_value::<LinearState>(value.clone()) else {
                return json!({});
            };
            if state.id.is_nil()
                || [&state.name, &state.r#type]
                    .iter()
                    .any(|text| text.len() > 1024 || text.contains('\0'))
            {
                return json!({});
            }
            Some(state)
        }
        None => return json!({}),
    };
    json!({"identifier":identifier,"state":state})
}
// Deliberately no Debug: imported text must not leak through an error/debug log.
#[derive(Clone, Serialize, Deserialize)]
pub struct ExistingToolSnapshot {
    pub external_id: Uuid,
    pub canonical_url: String,
    pub title: String,
    pub body_markdown: String,
    pub remote_updated_at: Option<DateTime<Utc>>,
    pub coverage: SourceCoverage,
    pub omission_reasons: Vec<OmissionReason>,
    pub metadata: SourceMetadata,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExistingReadErrorCode {
    RemoteAuthentication,
    RemotePermission,
    RemoteNotFound,
    RemoteRateLimit,
    RemoteTimeout,
    RemoteTransport,
    RemoteUnavailable,
    RemoteResponseInvalid,
    RemoteResponseTooLarge,
    RemoteIdentityMismatch,
    RemoteChangedDuringRead,
    RemoteRedirectBlocked,
}
impl ExistingReadErrorCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RemoteAuthentication => "remote_authentication",
            Self::RemotePermission => "remote_permission",
            Self::RemoteNotFound => "remote_not_found",
            Self::RemoteRateLimit => "remote_rate_limit",
            Self::RemoteTimeout => "remote_timeout",
            Self::RemoteTransport => "remote_transport",
            Self::RemoteUnavailable => "remote_unavailable",
            Self::RemoteResponseInvalid => "remote_response_invalid",
            Self::RemoteResponseTooLarge => "remote_response_too_large",
            Self::RemoteIdentityMismatch => "remote_identity_mismatch",
            Self::RemoteChangedDuringRead => "remote_changed_during_read",
            Self::RemoteRedirectBlocked => "remote_redirect_blocked",
        }
    }
    #[must_use]
    pub const fn retryable(self) -> bool {
        matches!(
            self,
            Self::RemoteRateLimit
                | Self::RemoteTimeout
                | Self::RemoteTransport
                | Self::RemoteUnavailable
        )
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingReadFailure {
    pub code: ExistingReadErrorCode,
    pub retry_after_seconds: Option<u32>,
    pub retryable: bool,
}
impl ExistingReadFailure {
    #[must_use]
    pub const fn new(code: ExistingReadErrorCode) -> Self {
        Self {
            code,
            retry_after_seconds: None,
            retryable: code.retryable(),
        }
    }
}
impl std::fmt::Display for ExistingReadFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code.as_str())
    }
}
impl std::error::Error for ExistingReadFailure {}
// A failure contains a closed code only, never provider text or credentials.
pub enum ExistingReadOutcome {
    Available(Box<ExistingToolSnapshot>),
    Unavailable { code: ExistingReadErrorCode },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    ToolSourceObservation,
    PublicationObservation,
}
impl SourceKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ToolSourceObservation => "tool_source_observation",
            Self::PublicationObservation => "publication_observation",
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SourceFreshness {
    pub is_current: bool,
    pub eligible: bool,
    pub reasons: Vec<String>,
    pub current_observation_id: Option<Uuid>,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub last_check_status: Option<String>,
    pub last_check_error_code: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SourceObservation {
    pub source_kind: SourceKind,
    pub public_id: Uuid,
    pub reference_public_id: Option<Uuid>,
    pub publication_public_id: Option<Uuid>,
    pub source_project_public_id: Uuid,
    pub provider: ToolProvider,
    pub object_kind: String,
    pub external_id: Option<Uuid>,
    pub canonical_url: String,
    pub version: i32,
    pub observed_at: DateTime<Utc>,
    pub remote_updated_at: Option<DateTime<Utc>>,
    pub title: String,
    pub excerpt: String,
    pub availability: String,
    pub content_hash: String,
    pub snapshot_hash: String,
    pub projection_version: String,
    pub trust: String,
    pub mandatory: bool,
    pub coverage: SourceCoverage,
    pub omission_reasons: Vec<String>,
    pub freshness: SourceFreshness,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SourceObservationDetail {
    pub observation: SourceObservation,
    pub body_markdown: String,
    pub metadata: serde_json::Value,
}
#[derive(Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ToolSourceReference {
    pub public_id: Uuid,
    pub project_id: Uuid,
    pub provider: String,
    pub object_kind: String,
    pub external_id: Uuid,
    pub canonical_url: String,
    pub connection_id: Uuid,
    pub connection_revision: i32,
    pub status: String,
    pub revision: i32,
    pub current_observation_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub last_check_status: String,
    pub last_check_error_code: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ToolSourceDetail {
    pub reference: ToolSourceReference,
    pub observation: SourceObservation,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SourceCommandResult {
    pub reference: ToolSourceReference,
    pub observation: SourceObservation,
    pub action: String,
    pub effect: String,
    pub verification_status: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachToolSource {
    pub connection_id: Uuid,
    pub expected_connection_revision: i32,
    pub provider: ToolProvider,
    pub source: String,
    pub confirm_scope_sharing: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedSource {
    pub expected_revision: i32,
    pub expected_observation_id: Uuid,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RebindToolSource {
    pub expected_revision: i32,
    pub expected_observation_id: Uuid,
    pub connection_id: Uuid,
    pub expected_connection_revision: i32,
    pub confirm_scope_sharing: bool,
}
#[derive(Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListSources {
    pub provider: Option<ToolProvider>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}
#[derive(Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListObservations {
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}
#[derive(Serialize)]
pub struct ToolSources {
    pub items: Vec<ToolSourceDetail>,
    pub next_cursor: Option<String>,
    pub total_count: i64,
    pub active_count: i64,
    pub limit: i64,
}
#[derive(Serialize)]
pub struct SourceObservations {
    pub items: Vec<SourceObservation>,
    pub next_cursor: Option<String>,
    pub total_count: i64,
    pub limit: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn context_metadata_preserves_only_bounded_observed_issue_facts() {
        let state =
            json!({"id":"30000000-0000-4000-8000-000000000001","name":"En cours","type":"started"});
        let mut input = json!({"identifier":"PROD-42","state":state,"arbitrary_instructions":"[FICTIF] Do not forward"});
        input["state"]["extra"] = json!("[FICTIF] Not part of the projection");
        assert_eq!(
            context_metadata(ToolProvider::Linear, &input),
            json!({"identifier":"PROD-42","state":state})
        );
        assert_eq!(context_metadata(ToolProvider::Notion, &input), json!({}));
        assert_eq!(
            context_metadata(
                ToolProvider::Linear,
                &json!({"identifier":"PROD-42","state":null})
            ),
            json!({"identifier":"PROD-42","state":null})
        );
    }

    #[test]
    fn malformed_or_oversized_metadata_does_not_become_model_context() {
        let mut input = json!({"identifier":"PROD-42","state":{"id":"30000000-0000-4000-8000-000000000001","name":"é".repeat(512),"type":"started"}});
        assert_eq!(context_metadata(ToolProvider::Linear, &input), input);
        input["state"]["name"] = json!("é".repeat(513));
        assert_eq!(context_metadata(ToolProvider::Linear, &input), json!({}));
        for value in [
            json!({}),
            json!({"identifier":"https://arbitrary.invalid","state":null}),
            json!({"identifier":"PROD-42","state":{"id":"not-a-uuid","name":"En cours","type":"started"}}),
        ] {
            assert_eq!(context_metadata(ToolProvider::Linear, &value), json!({}));
        }
    }
}
