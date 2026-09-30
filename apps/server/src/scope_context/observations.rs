//! Bounded projection of exact observed data; no provider I/O or implicit truth.
use super::SourceCandidate;
use crate::{
    context::ContextCandidate,
    error::{AppError, AppResult},
    work_tools::sources::models::{SourceKind, context_metadata},
};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub(super) const MAX_SOURCES: usize = 20;
pub(super) const MAX_EXCERPT_BYTES: usize = 8192;
pub(super) const MAX_EXTERNAL_BYTES: usize = 65_536;

pub(crate) fn kind(value: &str) -> AppResult<SourceKind> {
    match value {
        "tool_source_observation" => Ok(SourceKind::ToolSourceObservation),
        "publication_observation" => Ok(SourceKind::PublicationObservation),
        _ => Err(AppError::Invalid(
            "Unknown external observation kind".into(),
        )),
    }
}

pub(super) fn truncate_utf8(value: &mut String, maximum: usize) -> bool {
    if value.len() <= maximum {
        return false;
    }
    let mut end = maximum;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    true
}

pub(super) async fn enrich(
    tx: &mut Transaction<'_, Postgres>,
    candidate: &mut ContextCandidate,
    source_kind: &str,
) -> AppResult<()> {
    let detail = crate::work_tools::sources::read::observation_in_tx(
        tx,
        kind(source_kind)?,
        candidate.version_public_id,
    )
    .await?;
    if !detail.observation.freshness.eligible {
        return Err(AppError::Conflict(
            "An observed source changed during context selection".into(),
        ));
    }
    let metadata = context_metadata(detail.observation.provider, &detail.metadata);
    let mut observation =
        serde_json::to_value(detail.observation).map_err(|e| AppError::Internal(e.to_string()))?;
    // The candidate owns its excerpt. Avoid sending a second, differently bounded copy.
    observation
        .as_object_mut()
        .expect("observation object")
        .remove("excerpt");
    candidate.statement = detail.body_markdown;
    let truncated = truncate_utf8(&mut candidate.statement, MAX_EXCERPT_BYTES);
    observation["excerpt_truncated"] = json!(truncated);
    observation["metadata"] = metadata;
    candidate.observation = Some(observation);
    Ok(())
}

pub(crate) async fn snapshot(
    tx: &mut Transaction<'_, Postgres>,
    source_kind: &str,
    public_id: Uuid,
) -> AppResult<(i64, i64, Value)> {
    let source_kind_value = kind(source_kind)?;
    let sql = match source_kind_value {
        SourceKind::ToolSourceObservation => {
            "select id,project_id from app.tool_source_observations where public_id=$1 and workspace_id=app.current_workspace_id()"
        }
        SourceKind::PublicationObservation => {
            "select o.id,j.project_id from app.publication_observations o join app.publication_jobs j on j.id=o.publication_job_id where o.public_id=$1 and o.workspace_id=app.current_workspace_id()"
        }
    };
    let (id, project_id) = sqlx::query_as(sql)
        .bind(public_id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(AppError::NotFound)?;
    let detail =
        crate::work_tools::sources::read::observation_in_tx(tx, source_kind_value, public_id)
            .await?;
    let metadata = context_metadata(detail.observation.provider, &detail.metadata);
    let mut value =
        serde_json::to_value(detail.observation).map_err(|e| AppError::Internal(e.to_string()))?;
    value["metadata"] = metadata;
    value["kind"] = json!(source_kind);
    value["project_id"] = value["source_project_public_id"].clone();
    Ok((id, project_id, value))
}

pub(super) async fn load(
    tx: &mut Transaction<'_, Postgres>,
    project_id: i64,
    scopes: &[i64],
    remaining_bytes: usize,
) -> AppResult<(Vec<SourceCandidate>, Value)> {
    let sql = format!("with observed as ({}), deduplicated as (
      select distinct on (project_id,provider,external_id) * from observed where project_id=any($1)
      order by project_id,provider,external_id,observed_at desc,source_kind,source_id desc
    ) select source_id,project_id,project_public_id,jsonb_build_object(
      'knowledge_public_id',object_public_id,'version_public_id',public_id,'version_number',version,
      'entry_type','external_observation','title',title,'statement',left(body_markdown,8192),'rationale','Observed external data; coverage is explicit',
      'node_key',scope_kind||'/external','source_kind',source_kind),count(*) over(),
      (select count(*) from observed where project_id=any($1)) from deduplicated order by (project_id=$2) desc,observed_at desc,source_kind,source_id desc limit 21",include_str!("observations.sql"));
    let rows: Vec<(
        i64,
        i64,
        Uuid,
        sqlx::types::Json<ContextCandidate>,
        i64,
        i64,
    )> = sqlx::query_as(&sql)
        .bind(scopes)
        .bind(project_id)
        .fetch_all(&mut **tx)
        .await?;
    let total = rows.first().map_or(0, |r| r.4);
    let before_dedup = rows.first().map_or(0, |r| r.5);
    let more = rows.len() > MAX_SOURCES;
    let mut sources = Vec::new();
    let mut bytes = 0;
    for (
        source_id,
        source_project_id,
        source_project_public_id,
        sqlx::types::Json(mut candidate),
        _,
        _,
    ) in rows.into_iter().take(MAX_SOURCES)
    {
        let source_kind = match candidate.source_kind {
            crate::context::ContextSourceKind::ToolSourceObservation => "tool_source_observation",
            crate::context::ContextSourceKind::PublicationObservation => "publication_observation",
            _ => {
                return Err(AppError::Internal(
                    "Unknown observed context source kind".into(),
                ));
            }
        };
        enrich(tx, &mut candidate, source_kind).await?;
        let size = serde_json::to_vec(&candidate)
            .map_err(|e| AppError::Internal(e.to_string()))?
            .len()
            + candidate
                .observation
                .as_ref()
                .map_or(0, |value| value.to_string().len())
            + 300;
        if bytes + size > MAX_EXTERNAL_BYTES || bytes + size > remaining_bytes {
            continue;
        }
        bytes += size;
        sources.push(SourceCandidate {
            source_id,
            source_project_id,
            source_project_public_id,
            source_kind,
            candidate,
        });
    }
    let omitted = total - i64::try_from(sources.len()).unwrap_or(i64::MAX);
    Ok((
        sources,
        json!({"external_eligible_objects":total,"external_duplicate_sources":before_dedup-total,"external_omitted_sources":omitted,"external_candidates_truncated":more,"max_external_sources":MAX_SOURCES,"max_external_excerpt_bytes":MAX_EXCERPT_BYTES,"max_external_bytes":MAX_EXTERNAL_BYTES}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_excerpt_limit_keeps_valid_utf8_and_reports_omission() {
        let mut value = "é".repeat(5000);
        assert!(truncate_utf8(&mut value, 8191));
        assert_eq!(value.len(), 8190);
        assert_eq!(value.chars().count(), 4095);
        assert!(!truncate_utf8(&mut value, 8191));
    }
}
