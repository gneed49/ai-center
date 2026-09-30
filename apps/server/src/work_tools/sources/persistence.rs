//! Append immutable evidence and update only the reference's verification state.
use super::{
    models::{
        ExistingToolSnapshot, PROJECTION_VERSION, SourceCommandResult, SourceCoverage, ToolProvider,
    },
    read,
};
use crate::{
    error::{AppError, AppResult},
    service::AppState,
};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Clone, sqlx::FromRow)]
pub(super) struct Reference {
    pub id: i64,
    pub public_id: Uuid,
    pub project_id: i64,
    pub provider: String,
    pub external_id: Uuid,
    pub canonical_url: String,
    pub connection_public_id: Uuid,
    pub status: String,
    pub revision: i32,
    pub observation_public_id: Uuid,
    pub observation_id: i64,
    pub version: i32,
    pub snapshot_hash: String,
    pub last_attempt_at: Option<DateTime<Utc>>,
}
const SELECT: &str = "select r.id,r.public_id,r.project_id,r.provider,r.external_id,r.canonical_url,c.public_id as connection_public_id,r.status,r.revision,o.public_id as observation_public_id,o.id as observation_id,o.version,o.snapshot_hash,r.last_attempt_at from app.tool_source_references r join app.work_tool_connections c on c.id=r.connection_id join app.tool_source_observations o on o.id=r.current_observation_id";
pub(super) async fn reference(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    lock: bool,
) -> AppResult<Reference> {
    sqlx::query_as(&format!(
        "{SELECT} where r.public_id=$1 and r.workspace_id=app.current_workspace_id() {}",
        if lock { "for update of r" } else { "" }
    ))
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(AppError::NotFound)
}
pub(super) async fn identity(
    tx: &mut Transaction<'_, Postgres>,
    project: i64,
    provider: ToolProvider,
    external: Uuid,
) -> AppResult<Option<Reference>> {
    Ok(sqlx::query_as(&format!("{SELECT} where r.project_id=$1 and r.provider=$2 and r.external_id=$3 and r.workspace_id=app.current_workspace_id() for update of r")).bind(project).bind(provider.as_str()).bind(external).fetch_optional(&mut **tx).await?)
}
pub(super) async fn alias(
    tx: &mut Transaction<'_, Postgres>,
    project: i64,
    connection: Uuid,
    identifier: &str,
) -> AppResult<Option<Reference>> {
    Ok(sqlx::query_as(&format!("{SELECT} where r.project_id=$1 and c.public_id=$2 and r.provider='linear' and o.metadata->>'identifier'=$3 and r.workspace_id=app.current_workspace_id() for update of r")).bind(project).bind(connection).bind(identifier).fetch_optional(&mut **tx).await?)
}
pub(super) async fn capacity(tx: &mut Transaction<'_, Postgres>, project: i64) -> AppResult<()> {
    let n:i64=sqlx::query_scalar("select count(*) from app.tool_source_references where project_id=$1 and workspace_id=app.current_workspace_id() and status='active'").bind(project).fetch_one(&mut **tx).await?;
    if n >= 200 {
        return Err(super::admission::refusal(
            "source_limit_reached",
            "Ce projet contient déjà 200 sources actives. Retirez une source avant d’en rattacher une autre.",
            None,
            axum::http::StatusCode::CONFLICT,
        ));
    }
    Ok(())
}
pub(super) struct Observation {
    pub external_id: Uuid,
    pub canonical_url: String,
    pub remote_updated_at: Option<DateTime<Utc>>,
    pub title: String,
    pub body: String,
    pub availability: &'static str,
    pub coverage: &'static str,
    pub omissions: Value,
    pub metadata: Value,
    pub content_hash: String,
    pub snapshot_hash: String,
}
impl Observation {
    pub fn available(snapshot: ExistingToolSnapshot, provider: ToolProvider) -> AppResult<Self> {
        let coverage = match snapshot.coverage {
            SourceCoverage::Complete => "complete",
            SourceCoverage::Partial => "partial",
            SourceCoverage::None => {
                return Err(AppError::Internal(
                    "Available source has no coverage".into(),
                ));
            }
        };
        Self::build(
            provider,
            snapshot.external_id,
            snapshot.canonical_url,
            snapshot.remote_updated_at,
            snapshot.title,
            snapshot.body_markdown,
            "available",
            coverage,
            json!(snapshot.omission_reasons),
            json!(snapshot.metadata),
        )
    }
    pub fn unavailable(reference: &Reference, provider: ToolProvider) -> AppResult<Self> {
        Self::build(
            provider,
            reference.external_id,
            reference.canonical_url.clone(),
            None,
            String::new(),
            String::new(),
            "unavailable",
            "none",
            json!([]),
            json!({}),
        )
    }
    #[allow(clippy::too_many_arguments, clippy::unnecessary_wraps)]
    fn build(
        provider: ToolProvider,
        external_id: Uuid,
        canonical_url: String,
        remote_updated_at: Option<DateTime<Utc>>,
        title: String,
        body: String,
        availability: &'static str,
        coverage: &'static str,
        omissions: Value,
        metadata: Value,
    ) -> AppResult<Self> {
        let content_hash = crate::idempotency::hash_json(
            &json!({"title":title,"body_markdown":body,"state":metadata.get("state")}),
        );
        let snapshot_hash = crate::idempotency::hash_json(
            &json!({"content_hash":content_hash,"provider":provider,"external_id":external_id,"canonical_url":canonical_url,"remote_updated_at":remote_updated_at,"availability":availability,"coverage":coverage,"omission_reasons":omissions,"metadata":metadata,"projection_version":PROJECTION_VERSION}),
        );
        Ok(Self {
            external_id,
            canonical_url,
            remote_updated_at,
            title,
            body,
            availability,
            coverage,
            omissions,
            metadata,
            content_hash,
            snapshot_hash,
        })
    }
}
#[allow(clippy::too_many_arguments)]
pub(super) async fn persist(
    tx: &mut Transaction<'_, Postgres>,
    state: &AppState,
    project: i64,
    provider: ToolProvider,
    connection: i64,
    connection_revision: i32,
    existing: Option<&Reference>,
    observation: Observation,
    attempt_at: DateTime<Utc>,
    action: &str,
    error_code: Option<&str>,
) -> AppResult<SourceCommandResult> {
    let (reference_id, public_id, version, mut observation_id) = if let Some(r) = existing {
        (r.id, r.public_id, r.version + 1, r.observation_id)
    } else {
        capacity(tx, project).await?;
        let public_id = Uuid::new_v4();
        let id:i64=sqlx::query_scalar("insert into app.tool_source_references(public_id,workspace_id,project_id,provider,object_kind,external_id,canonical_url,connection_id,connection_revision,created_by_actor_id,last_attempt_at) values($1,app.current_workspace_id(),$2,$3,$4,$5,$6,$7,$8,$9,$10) returning id")
            .bind(public_id).bind(project).bind(provider.as_str()).bind(provider.object_kind()).bind(observation.external_id).bind(&observation.canonical_url).bind(connection).bind(connection_revision).bind(state.actor_id).bind(attempt_at).fetch_one(&mut **tx).await?;
        (id, public_id, 1, 0)
    };
    let changed = existing.is_none_or(|r| r.snapshot_hash != observation.snapshot_hash);
    if changed {
        observation_id=sqlx::query_scalar("insert into app.tool_source_observations(workspace_id,project_id,reference_id,version,provider,object_kind,external_id,canonical_url,connection_id,connection_revision,remote_updated_at,title,body_markdown,availability,coverage,omission_reasons,projection_version,content_hash,snapshot_hash,metadata) values(app.current_workspace_id(),$1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19) returning id")
            .bind(project).bind(reference_id).bind(version).bind(provider.as_str()).bind(provider.object_kind()).bind(observation.external_id).bind(&observation.canonical_url).bind(connection).bind(connection_revision).bind(observation.remote_updated_at).bind(observation.title).bind(observation.body).bind(observation.availability).bind(observation.coverage).bind(observation.omissions).bind(PROJECTION_VERSION).bind(observation.content_hash).bind(observation.snapshot_hash).bind(observation.metadata).fetch_one(&mut **tx).await?;
    }
    let verification = if observation.availability == "unavailable" {
        "unavailable"
    } else if observation.coverage == "partial" {
        "partial"
    } else {
        "available"
    };
    // SQL owns graph invalidation/events. A mere unchanged check creates neither.
    sqlx::query("update app.tool_source_references set current_observation_id=$2,connection_id=$3,connection_revision=$4,canonical_url=$5,status='active',revision=revision+$6,updated_at=clock_timestamp(),last_attempt_at=$7,last_checked_at=clock_timestamp(),last_check_status=$8,last_check_error_code=$9 where id=$1")
        .bind(reference_id).bind(observation_id).bind(connection).bind(connection_revision).bind(observation.canonical_url).bind(i32::from(existing.is_some())).bind(attempt_at).bind(verification).bind(error_code).execute(&mut **tx).await?;
    let effect = if action == "rebind" {
        "rebound"
    } else if existing.is_none() {
        "created"
    } else if changed {
        "changed"
    } else {
        "unchanged"
    };
    result(tx, public_id, action, effect, verification).await
}
pub(super) async fn result(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    action: &str,
    effect: &str,
    verification: &str,
) -> AppResult<SourceCommandResult> {
    let detail = read::detail_in_tx(tx, id).await?;
    Ok(SourceCommandResult {
        reference: detail.reference,
        observation: detail.observation,
        action: action.into(),
        effect: effect.into(),
        verification_status: verification.into(),
    })
}
