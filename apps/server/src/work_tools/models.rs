use chrono::{DateTime, Utc};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Connection {
    pub public_id: Uuid,
    pub provider: String,
    pub name: String,
    pub enabled: bool,
    pub revision: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Serialize)]
#[allow(clippy::struct_excessive_bools)] // Independent advertised capabilities, not a lifecycle state.
pub struct Capability {
    pub provider: &'static str,
    pub create: bool,
    pub read: bool,
    pub reconcile: bool,
    pub update: bool,
}
#[derive(Serialize)]
pub struct Settings {
    pub limits: super::reliability::Limits,
    pub usage: super::reliability::Usage,
    pub storage_available: bool,
    pub connections: Vec<Connection>,
    pub capabilities: Vec<Capability>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveConnection {
    pub id: Uuid,
    pub provider: String,
    pub name: String,
    pub expected_revision: i32,
    #[serde(default, deserialize_with = "secret_input", skip_serializing)]
    pub api_key: Option<SecretString>,
}
fn secret_input<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<SecretString>, D::Error> {
    Option::<String>::deserialize(d).map(|value| value.map(SecretString::from))
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisableConnection {
    pub expected_revision: i32,
}
#[derive(Serialize, Deserialize)]
pub struct ConnectionTest {
    pub ok: bool,
    pub code: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishArtifact {
    pub version_id: Uuid,
    pub connection_id: Uuid,
    pub expected_provider: String,
    pub expected_target_id: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconcilePublication {
    pub external_id: String,
}
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Publication {
    pub public_id: Uuid,
    pub artifact_id: Uuid,
    pub version_id: Uuid,
    pub connection_id: Uuid,
    pub provider: String,
    pub target_id: String,
    pub status: String,
    pub external_id: Option<String>,
    pub external_url: Option<String>,
    pub error_code: Option<String>,
    pub attempt_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Serialize)]
pub struct Publications {
    pub items: Vec<Publication>,
    pub limit: i64,
    pub offset: i64,
}
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ListPublications {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Observation {
    pub public_id: Uuid,
    pub observed_at: DateTime<Utc>,
    pub observation_kind: String,
    pub external_id: String,
    pub external_url: String,
    pub remote_updated_at: Option<String>,
    pub snapshot: Value,
}
#[derive(Serialize)]
pub struct PublicationDetail {
    pub publication: Publication,
    pub observations: Vec<Observation>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Receipt {
    pub external_id: String,
    pub external_url: String,
    pub target_id: String,
    pub title: String,
    pub body_markdown: String,
    pub remote_updated_at: Option<String>,
    pub complete: bool,
}
pub(crate) struct Credential {
    pub provider: String,
    pub revision: i32,
    pub secret: SecretString,
}
#[derive(sqlx::FromRow)]
pub(crate) struct JobInput {
    pub id: i64,
    pub project_id: i64,
    pub public_id: Uuid,
    pub connection_public_id: Uuid,
    pub connection_revision: i32,
    pub provider: String,
    pub target_id: String,
    pub title: String,
    pub body_markdown: String,
    pub status: String,
    pub external_id: Option<String>,
    pub external_url: Option<String>,
}
