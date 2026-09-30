//! Explicit publication commands. Preview bodies never contain a future job marker.
use super::Publication;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewTickets {
    pub version_id: Uuid,
    pub connection_id: Uuid,
    pub expected_provider: String,
    pub expected_target_id: String,
    pub ticket_indexes: Vec<i16>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishTickets {
    pub version_id: Uuid,
    pub connection_id: Uuid,
    pub expected_provider: String,
    pub expected_target_id: String,
    pub ticket_indexes: Vec<i16>,
    pub preview_fingerprint: String,
    pub prior_publications_fingerprint: String,
    #[serde(default)]
    pub confirm_additional_issues: bool,
}
impl PublishTickets {
    #[must_use]
    pub fn preparation(&self) -> PreviewTickets {
        PreviewTickets {
            version_id: self.version_id,
            connection_id: self.connection_id,
            expected_provider: self.expected_provider.clone(),
            expected_target_id: self.expected_target_id.clone(),
            ticket_indexes: self.ticket_indexes.clone(),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TicketCoverageQuery {
    pub version_id: Uuid,
    pub provider: String,
    pub target_id: String,
}
#[derive(Serialize)]
pub struct TicketCoverageItem {
    pub source_ticket_index: i16,
    pub title: String,
    pub existing_publication: Option<Publication>,
}
#[derive(Serialize)]
pub struct TicketCoverage {
    pub artifact_id: Uuid,
    pub source_version_number: i32,
    pub version_id: Uuid,
    pub provider: String,
    pub target_id: String,
    pub items: Vec<TicketCoverageItem>,
}
#[derive(Serialize)]
pub struct TicketPreviewItem {
    pub source_ticket_index: i16,
    pub title: String,
    pub business_body_markdown: String,
    pub existing_publication: Option<Publication>,
}
#[derive(Serialize)]
pub struct TicketCapacity {
    pub available_pending: i64,
    pub available_hourly: i64,
    pub max_pending: i64,
    pub max_per_hour: i64,
}
#[derive(Serialize)]
pub struct TicketPreview {
    pub artifact_id: Uuid,
    pub version_id: Uuid,
    pub source_version_number: i32,
    pub content_hash: String,
    pub provider: String,
    pub target_id: String,
    pub connection_id: Uuid,
    pub connection_revision: i32,
    pub ticket_indexes: Vec<i16>,
    pub requested_count: usize,
    pub new_count: usize,
    pub existing_count: usize,
    pub items: Vec<TicketPreviewItem>,
    pub prior_publications: Vec<Publication>,
    pub prior_publications_fingerprint: String,
    pub requires_additional_confirmation: bool,
    pub preview_fingerprint: String,
    pub capacity: TicketCapacity,
}
#[derive(Serialize, Deserialize)]
pub struct TicketPublicationResult {
    pub artifact_id: Uuid,
    pub source_version_number: i32,
    pub version_id: Uuid,
    pub provider: String,
    pub target_id: String,
    pub publications: Vec<Publication>,
    pub created_count: usize,
    pub existing_count: usize,
}
