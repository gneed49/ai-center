use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, FromRow)]
pub struct TeamMember {
    pub public_id: Uuid,
    pub actor_id: Uuid,
    pub role: String,
    pub invitation_status: String,
    pub accepted_at: Option<DateTime<Utc>>,
    pub display_name: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Invitation {
    pub public_id: Uuid,
    pub role: String,
    pub label: String,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateInvitation {
    pub public_id: Uuid,
    pub token_hash: String,
    pub role: String,
    #[serde(default)]
    pub label: String,
    pub expires_in_days: u32,
}

// Deliberately neither Debug nor Serialize: presented bearer tokens must not
// be logged or copied to a durable command receipt.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewInvitation {
    pub token: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptInvitation {
    pub token: String,
    pub display_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetProfile {
    pub display_name: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct InvitationPreview {
    pub public_id: Uuid,
    pub workspace_public_id: Uuid,
    pub company_name: String,
    pub role: String,
    pub expires_at: DateTime<Utc>,
}
