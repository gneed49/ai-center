//! Local exact snapshots only. None of these reads contacts a work tool.
use super::models::{
    ListObservations, ListSources, SourceCoverage, SourceFreshness, SourceKind, SourceObservation,
    SourceObservationDetail, SourceObservations, ToolProvider, ToolSourceDetail,
    ToolSourceReference, ToolSources,
};
use crate::{
    error::{AppError, AppResult},
    service::AppState,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub(super) const REFERENCE_SELECT: &str = "select r.public_id,p.public_id as project_id,r.provider,r.object_kind,r.external_id,r.canonical_url,c.public_id as connection_id,r.connection_revision,r.status,r.revision,o.public_id as current_observation_id,r.created_at,r.updated_at,r.last_attempt_at,r.last_checked_at,coalesce(r.last_check_status,'failed') as last_check_status,r.last_check_error_code from app.tool_source_references r join app.projects p on p.id=r.project_id join app.work_tool_connections c on c.id=r.connection_id join app.tool_source_observations o on o.id=r.current_observation_id";
#[derive(sqlx::FromRow)]
#[allow(clippy::struct_excessive_bools)] // Independent SQL authority facts, not combinatorial states.
struct ObservationRow {
    public_id: Uuid,
    reference_public_id: Option<Uuid>,
    publication_public_id: Option<Uuid>,
    source_project_public_id: Uuid,
    provider: String,
    object_kind: String,
    external_id: Option<Uuid>,
    canonical_url: String,
    version: i32,
    observed_at: DateTime<Utc>,
    remote_updated_at: Option<String>,
    title: String,
    body_markdown: String,
    availability: String,
    coverage: String,
    omission_reasons: Value,
    projection_version: String,
    content_hash: String,
    snapshot_hash: String,
    metadata: Value,
    is_current: bool,
    eligible: bool,
    reference_active: bool,
    connection_enabled: bool,
    read_allowed: bool,
    connection_verified: bool,
    project_active: bool,
    current_observation_id: Option<Uuid>,
    last_checked_at: Option<DateTime<Utc>>,
    last_check_status: Option<String>,
    last_check_error_code: Option<String>,
}
const TOOL_OBSERVATION:&str="select o.public_id,r.public_id as reference_public_id,null::uuid as publication_public_id,p.public_id as source_project_public_id,o.provider,o.object_kind,o.external_id,o.canonical_url,o.version,o.observed_at,o.remote_updated_at::text,o.title,case when $2 then o.body_markdown else '' end as body_markdown,o.availability,o.coverage,o.omission_reasons,o.projection_version,o.content_hash,o.snapshot_hash,o.metadata,
    r.current_observation_id=o.id as is_current,app.tool_source_observation_current(o.id) as eligible,r.status='active' as reference_active,c.enabled as connection_enabled,c.allow_existing_reads as read_allowed,c.revision=r.connection_revision as connection_verified,p.status='active' as project_active,head.public_id as current_observation_id,r.last_checked_at,r.last_check_status,r.last_check_error_code
    from app.tool_source_observations o join app.tool_source_references r on r.id=o.reference_id join app.projects p on p.id=o.project_id join app.work_tool_connections c on c.id=r.connection_id left join app.tool_source_observations head on head.id=r.current_observation_id
    where o.public_id=$1 and o.workspace_id=app.current_workspace_id()";
const PUBLICATION_OBSERVATION:&str="select o.public_id,null::uuid as reference_public_id,o.publication_public_id,p.public_id as source_project_public_id,o.provider,o.object_kind,o.external_id,o.canonical_url,o.version,o.observed_at,o.remote_updated_at,o.title,case when $2 then o.body_markdown else '' end as body_markdown,o.availability,o.coverage,o.omission_reasons,o.projection_version,o.content_hash,o.snapshot_hash,o.metadata,
    o.is_current,app.publication_observation_current(o.id) as eligible,true as reference_active,c.enabled as connection_enabled,true as read_allowed,coalesce(o.authority_connection_id=c.id and o.authority_connection_revision=c.revision,false) as connection_verified,p.status='active' as project_active,latest.canonical_observation_public_id as current_observation_id,latest.observed_at as last_checked_at,case when latest.availability='unavailable' then 'unavailable' when latest.coverage='partial' then 'partial' else 'available' end as last_check_status,j.error_code as last_check_error_code
    from app.publication_source_observations o join app.publication_jobs j on j.id=o.publication_job_id join app.projects p on p.id=o.project_id join app.work_tool_connections c on c.id=j.connection_id left join app.publication_source_observations latest on latest.id=o.latest_observation_id
    where o.public_id=$1 and o.workspace_id=app.current_workspace_id()";
fn decode<T: for<'a> Deserialize<'a>>(value: Value) -> AppResult<T> {
    serde_json::from_value(value)
        .map_err(|_| AppError::Internal("Invalid persisted source contract".into()))
}
fn prefix(value: &str, limit: usize) -> String {
    let mut end = value.len().min(limit);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].into()
}
impl ObservationRow {
    fn into_detail(self, kind: SourceKind) -> AppResult<SourceObservationDetail> {
        let mut reasons = Vec::new();
        for (failed, reason) in [
            (!self.is_current, "historical"),
            (!self.reference_active, "detached"),
            (self.availability != "available", "unavailable"),
            (!self.connection_enabled, "connection_disabled"),
            (!self.read_allowed, "read_capability_disabled"),
            (!self.connection_verified, "connection_unverified"),
            (!self.project_active, "project_inactive"),
            (self.external_id.is_none(), "invalid_identity"),
        ] {
            if failed {
                reasons.push(reason.to_owned());
            }
        }
        // PostgreSQL timestamp text uses a space and a short timezone offset.
        let remote = self
            .remote_updated_at
            .as_deref()
            .and_then(|raw| {
                DateTime::parse_from_rfc3339(raw)
                    .or_else(|_| DateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S%.f%#z"))
                    .ok()
            })
            .map(|date| date.with_timezone(&Utc));
        let mut omissions: Vec<String> = decode(self.omission_reasons)?;
        let mut coverage: SourceCoverage = decode(Value::String(self.coverage))?;
        if remote.is_none() && self.availability == "available" {
            if !omissions
                .iter()
                .any(|reason| reason == "remote_date_unavailable")
            {
                omissions.push("remote_date_unavailable".into());
            }
            if coverage == SourceCoverage::Complete {
                coverage = SourceCoverage::Partial;
            }
        }
        Ok(SourceObservationDetail {
            observation: SourceObservation {
                source_kind: kind,
                public_id: self.public_id,
                reference_public_id: self.reference_public_id,
                publication_public_id: self.publication_public_id,
                source_project_public_id: self.source_project_public_id,
                provider: decode(Value::String(self.provider))?,
                object_kind: self.object_kind,
                external_id: self.external_id,
                canonical_url: self.canonical_url,
                version: self.version,
                observed_at: self.observed_at,
                remote_updated_at: remote,
                title: self.title,
                excerpt: prefix(&self.body_markdown, 8192),
                availability: self.availability,
                content_hash: self.content_hash,
                snapshot_hash: self.snapshot_hash,
                projection_version: self.projection_version,
                trust: "observed_external".into(),
                mandatory: false,
                coverage,
                omission_reasons: omissions,
                freshness: SourceFreshness {
                    is_current: self.is_current,
                    eligible: self.eligible && self.external_id.is_some(),
                    reasons,
                    current_observation_id: self.current_observation_id,
                    last_checked_at: self.last_checked_at,
                    last_check_status: self.last_check_status,
                    last_check_error_code: self.last_check_error_code,
                },
            },
            body_markdown: self.body_markdown,
            metadata: self.metadata,
        })
    }
}
/// Shared exact projection for pipeline/provenance; caller keeps its RLS transaction.
pub(crate) async fn observation_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    kind: SourceKind,
    id: Uuid,
) -> AppResult<SourceObservationDetail> {
    observation(tx, kind, id, true).await
}
async fn observation(
    tx: &mut Transaction<'_, Postgres>,
    kind: SourceKind,
    id: Uuid,
    body: bool,
) -> AppResult<SourceObservationDetail> {
    let sql = match kind {
        SourceKind::ToolSourceObservation => TOOL_OBSERVATION,
        SourceKind::PublicationObservation => PUBLICATION_OBSERVATION,
    };
    sqlx::query_as::<_, ObservationRow>(sql)
        .bind(id)
        .bind(body)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(AppError::NotFound)?
        .into_detail(kind)
}
pub async fn observation_detail(
    state: &AppState,
    kind: SourceKind,
    id: Uuid,
) -> AppResult<SourceObservationDetail> {
    let mut tx = state.begin_request().await?;
    let result = observation_in_tx(&mut tx, kind, id).await?;
    tx.commit().await?;
    Ok(result)
}
pub(crate) async fn reference_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> AppResult<ToolSourceReference> {
    sqlx::query_as(&format!(
        "{REFERENCE_SELECT} where r.public_id=$1 and r.workspace_id=app.current_workspace_id()"
    ))
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(AppError::NotFound)
}
pub(crate) async fn detail_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> AppResult<ToolSourceDetail> {
    let reference = reference_in_tx(tx, id).await?;
    let observation = observation_in_tx(
        tx,
        SourceKind::ToolSourceObservation,
        reference.current_observation_id,
    )
    .await?
    .observation;
    Ok(ToolSourceDetail {
        reference,
        observation,
    })
}
pub async fn detail(state: &AppState, id: Uuid) -> AppResult<ToolSourceDetail> {
    let mut tx = state.begin_request().await?;
    let result = detail_in_tx(&mut tx, id).await?;
    tx.commit().await?;
    Ok(result)
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    scope: String,
    created_at: DateTime<Utc>,
    public_id: Uuid,
}
fn limit(input: Option<i64>) -> AppResult<i64> {
    match input.unwrap_or(25) {
        value @ 1..=50 => Ok(value),
        _ => Err(AppError::Invalid(
            "Choisissez une page de 1 à 50 sources.".into(),
        )),
    }
}
fn cursor(input: Option<&str>, scope: &str) -> AppResult<Option<Cursor>> {
    let Some(input) = input else {
        return Ok(None);
    };
    let invalid = || AppError::Invalid("Le curseur de sources n’est plus valide.".into());
    if input.len() > 1024 {
        return Err(invalid());
    }
    let decoded: Cursor =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(input).map_err(|_| invalid())?)
            .map_err(|_| invalid())?;
    if decoded.scope != scope || decoded.public_id.is_nil() {
        return Err(invalid());
    }
    Ok(Some(decoded))
}
fn encode(cursor: &Cursor) -> AppResult<String> {
    Ok(URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&cursor)
            .map_err(|_| AppError::Internal("Source cursor unavailable".into()))?,
    ))
}
pub async fn list(state: &AppState, project: Uuid, input: ListSources) -> AppResult<ToolSources> {
    let limit = limit(input.limit)?;
    let status = input.status.as_deref().unwrap_or("active");
    if !matches!(status, "active" | "detached" | "all") {
        return Err(AppError::Invalid("État de source inconnu.".into()));
    }
    let provider = input.provider.map(ToolProvider::as_str);
    let scope = format!("{project}:{provider:?}:{status}");
    let cursor = cursor(input.cursor.as_deref(), &scope)?;
    let mut tx = state.begin_request().await?;
    let internal:i64=sqlx::query_scalar("select id from app.projects where public_id=$1 and workspace_id=app.current_workspace_id()").bind(project).fetch_optional(&mut *tx).await?.ok_or(AppError::NotFound)?;
    let (total_count,active_count):(i64,i64)=sqlx::query_as("select count(*) filter(where ($3='all' or status=$3)),count(*) filter(where status='active') from app.tool_source_references where workspace_id=app.current_workspace_id() and project_id=$1 and ($2::text is null or provider=$2)").bind(internal).bind(provider).bind(status).fetch_one(&mut *tx).await?;
    let mut references:Vec<ToolSourceReference>=sqlx::query_as(&format!("{REFERENCE_SELECT} where r.workspace_id=app.current_workspace_id() and r.project_id=$1 and ($2::text is null or r.provider=$2) and ($3='all' or r.status=$3) and ($4::timestamptz is null or (r.created_at,r.public_id)<($4,$5)) order by r.created_at desc,r.public_id desc limit $6"))
        .bind(internal).bind(provider).bind(status).bind(cursor.as_ref().map(|c|c.created_at)).bind(cursor.as_ref().map(|c|c.public_id)).bind(limit+1).fetch_all(&mut *tx).await?;
    let more = references.len() > usize::try_from(limit).unwrap_or(50);
    references.truncate(usize::try_from(limit).unwrap_or(50));
    let next_cursor = if more {
        references
            .last()
            .map(|r| {
                encode(&Cursor {
                    scope,
                    created_at: r.created_at,
                    public_id: r.public_id,
                })
            })
            .transpose()?
    } else {
        None
    };
    let mut items = Vec::new();
    for reference in references {
        let observation = observation(
            &mut tx,
            SourceKind::ToolSourceObservation,
            reference.current_observation_id,
            false,
        )
        .await?
        .observation;
        items.push(ToolSourceDetail {
            reference,
            observation,
        });
    }
    tx.commit().await?;
    Ok(ToolSources {
        items,
        next_cursor,
        total_count,
        active_count,
        limit,
    })
}
pub async fn history(
    state: &AppState,
    id: Uuid,
    input: ListObservations,
) -> AppResult<SourceObservations> {
    let limit = limit(input.limit)?;
    let scope = format!("history:{id}");
    let cursor = cursor(input.cursor.as_deref(), &scope)?;
    let mut tx = state.begin_request().await?;
    let reference:i64=sqlx::query_scalar("select id from app.tool_source_references where public_id=$1 and workspace_id=app.current_workspace_id()").bind(id).fetch_optional(&mut *tx).await?.ok_or(AppError::NotFound)?;
    let total_count:i64=sqlx::query_scalar("select count(*) from app.tool_source_observations where reference_id=$1 and workspace_id=app.current_workspace_id()").bind(reference).fetch_one(&mut *tx).await?;
    let before: Option<i32> = if let Some(cursor) = cursor {
        Some(sqlx::query_scalar("select version from app.tool_source_observations where reference_id=$1 and public_id=$2 and workspace_id=app.current_workspace_id()")
            .bind(reference).bind(cursor.public_id).fetch_optional(&mut *tx).await?.ok_or_else(||AppError::Invalid("Le curseur d’historique n’est plus valide.".into()))?)
    } else {
        None
    };
    let mut rows:Vec<(Uuid,DateTime<Utc>)>=sqlx::query_as("select public_id,observed_at from app.tool_source_observations where reference_id=$1 and workspace_id=app.current_workspace_id() and ($2::integer is null or version<$2) order by version desc limit $3")
        .bind(reference).bind(before).bind(limit+1).fetch_all(&mut *tx).await?;
    let more = rows.len() > usize::try_from(limit).unwrap_or(50);
    rows.truncate(usize::try_from(limit).unwrap_or(50));
    let next_cursor = if more {
        rows.last()
            .map(|(id, date)| {
                encode(&Cursor {
                    scope,
                    created_at: *date,
                    public_id: *id,
                })
            })
            .transpose()?
    } else {
        None
    };
    let mut items = Vec::new();
    for (id, _) in rows {
        items.push(
            observation(&mut tx, SourceKind::ToolSourceObservation, id, false)
                .await?
                .observation,
        );
    }
    tx.commit().await?;
    Ok(SourceObservations {
        items,
        next_cursor,
        total_count,
        limit,
    })
}
