//! External work provenance. These envelopes describe observations; they never
//! run tools, create GitHub objects, or validate evidence automatically.
use super::{ProjectScope, ReferenceRecord, load_latest_observation};
use crate::{
    error::{AppError, AppResult},
    service::AppState,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalTrackingInput {
    pub context_pack_id: Uuid,
    pub transmission_confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::FromRow)]
pub struct ExternalTrackingEvent {
    pub public_id: Uuid,
    pub event_type: String,
    pub sequence_number: i32,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::FromRow)]
pub struct ExternalTrackingArtifact {
    pub public_id: Uuid,
    pub reference: String,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExternalTrackingView {
    pub task_public_id: Uuid,
    pub execution_public_id: Uuid,
    pub context_pack_public_id: Uuid,
    pub context_pack_version: i32,
    pub context_pack_hash: String,
    pub context_pack_current: bool,
    pub status: String,
    pub observed_result: Value,
    pub artifacts: Vec<ExternalTrackingArtifact>,
    pub events: Vec<ExternalTrackingEvent>,
}

#[derive(sqlx::FromRow)]
struct TrackingRow {
    execution_id: i64,
    task_public_id: Uuid,
    execution_public_id: Uuid,
    context_pack_public_id: Uuid,
    context_pack_version: i32,
    context_pack_hash: String,
    context_pack_current: bool,
    status: String,
    observed_result: Value,
}

/// Check both tenant/project ownership and current graph before claiming an
/// external transmission. Locking the project prevents concurrent revisions
/// from making this declaration stale before its final transaction commits.
pub(super) async fn validate_pack(
    tx: &mut Transaction<'_, Postgres>,
    scope: &ProjectScope,
    input: &ExternalTrackingInput,
) -> AppResult<(i64, i64, String)> {
    if !input.transmission_confirmed {
        return Err(AppError::Invalid(
            "explicit confirmation of the transmitted ContextPack is required".into(),
        ));
    }
    let pack: Option<(i64, i64, String, bool)> = sqlx::query_as(
        "select pack.id, contract.id, pack.content_hash,
                pack.status = 'current' and pack.source_graph_version = project.graph_version
         from app.context_packs pack
         join app.projects project on project.id=pack.project_id and project.workspace_id=pack.workspace_id
         join app.deliverable_contracts contract on contract.contract_key=pack.task_kind and contract.template_id=project.template_id
         where pack.public_id=$1 and pack.project_id=$2 and pack.workspace_id=$3
         for share of project, pack",
    ).bind(input.context_pack_id).bind(scope.project_id).bind(scope.workspace_id).fetch_optional(&mut **tx).await?;
    let (pack_id, contract_id, hash, current) = pack.ok_or(AppError::NotFound)?;
    if !current {
        return Err(AppError::Conflict(
            "ContextPack is stale; transmit a newly compiled pack before linking new external work"
                .into(),
        ));
    }
    Ok((pack_id, contract_id, hash))
}

pub(super) async fn ensure_tracking(
    tx: &mut Transaction<'_, Postgres>,
    state: &AppState,
    reference: &ReferenceRecord,
    input: &ExternalTrackingInput,
) -> AppResult<()> {
    let scope = ProjectScope {
        workspace_id: reference.workspace_id,
        project_id: reference.project_id,
    };
    let (pack_id, contract_id, hash) = validate_pack(tx, &scope, input).await?;
    // The caller holds the reference row lock, serializing distinct command
    // keys that declare the same reference/pack pair.
    let existing: bool = sqlx::query_scalar(
        "select exists(select 1 from app.edges edge join app.tasks task on task.public_id=edge.target_public_id
         and task.project_id=edge.project_id and task.workspace_id=edge.workspace_id
         where edge.source_public_id=$1 and edge.source_kind='external_reference' and edge.target_kind='task'
         and edge.edge_type='tracked_by' and edge.status='confirmed'
         and task.context_pack_id=$2 and edge.project_id=$3 and edge.workspace_id=$4)",
    ).bind(reference.public_id).bind(pack_id).bind(reference.project_id).bind(reference.workspace_id).fetch_one(&mut **tx).await?;
    if existing {
        return Ok(());
    }
    let (task_id, task_public_id): (i64, Uuid) = sqlx::query_as(
        "insert into app.tasks(workspace_id,project_id,context_pack_id,contract_id,title,status)
         values($1,$2,$3,$4,$5,'running') returning id,public_id",
    )
    .bind(reference.workspace_id)
    .bind(reference.project_id)
    .bind(pack_id)
    .bind(contract_id)
    .bind(format!(
        "Travail externe observé : {}",
        reference.display_title
    ))
    .fetch_one(&mut **tx)
    .await?;
    sqlx::query("insert into app.executions(workspace_id,project_id,task_id,executor_key,status,result,started_at)
        values($1,$2,$3,'github-observation','running',$4,now())")
        .bind(reference.workspace_id).bind(reference.project_id).bind(task_id)
        .bind(json!({"mode":"external_observation","transmission":"user_confirmed","context_pack_hash":hash,"evidence_validation":"human_required"}))
        .execute(&mut **tx).await?;
    for (kind, public_id) in [
        ("context_pack", input.context_pack_id),
        ("external_reference", reference.public_id),
    ] {
        sqlx::query("insert into app.edges(workspace_id,project_id,source_kind,source_public_id,target_kind,target_public_id,edge_type,provenance,created_by_actor_id)
            values($1,$2,$3,$4,'task',$5,'tracked_by',$6,$7) on conflict(project_id,source_public_id,target_public_id,edge_type) do nothing")
            .bind(reference.workspace_id).bind(reference.project_id).bind(kind).bind(public_id).bind(task_public_id)
            .bind(json!({"mode":"external_observation","transmission":"user_confirmed","context_pack_hash":hash})).bind(state.actor_id).execute(&mut **tx).await?;
    }
    sqlx::query("insert into app.audit_events(workspace_id,project_id,actor_id,action,object_kind,object_public_id,after_state)
        values($1,$2,$3,'external_tracking.declared','task',$4,$5)")
        .bind(reference.workspace_id).bind(reference.project_id).bind(state.actor_id).bind(task_public_id)
        .bind(json!({"context_pack_id":input.context_pack_id,"context_pack_hash":hash,"external_reference_id":reference.public_id,"transmission":"user_confirmed"}))
        .execute(&mut **tx).await?;
    Ok(())
}

async fn execution_ids(
    tx: &mut Transaction<'_, Postgres>,
    reference: &ReferenceRecord,
) -> AppResult<Vec<i64>> {
    Ok(sqlx::query_scalar("select execution.id from app.edges edge join app.tasks task
        on task.public_id=edge.target_public_id and task.project_id=edge.project_id and task.workspace_id=edge.workspace_id
        join app.executions execution on execution.task_id=task.id and execution.project_id=task.project_id and execution.workspace_id=task.workspace_id
        where edge.source_kind='external_reference' and edge.target_kind='task' and edge.edge_type='tracked_by' and edge.status='confirmed'
        and edge.source_public_id=$1 and edge.project_id=$2 and edge.workspace_id=$3 and execution.executor_key='github-observation'
        order by execution.id for update of execution")
        .bind(reference.public_id).bind(reference.project_id).bind(reference.workspace_id).fetch_all(&mut **tx).await?)
}

pub(super) async fn observe(
    tx: &mut Transaction<'_, Postgres>,
    reference: &ReferenceRecord,
    command_id: Uuid,
    event_type: &str,
) -> AppResult<()> {
    let execution_ids = execution_ids(tx, reference).await?;
    if execution_ids.is_empty() {
        return Ok(());
    }
    let observation = load_latest_observation(
        tx,
        reference.id,
        reference.workspace_id,
        reference.project_id,
    )
    .await?;
    for execution_id in execution_ids {
        let mut payload = json!({"command_id":command_id,"external_reference_id":reference.public_id,"evidence_validation":"human_required"});
        if let Some(observation) = &observation {
            payload["observation_id"] = json!(observation.public_id);
            payload["observation_hash"] = json!(observation.content_hash);
            payload["sync_status"] = json!(reference.sync_status);
            payload["head_sha"] = observation
                .observed_state
                .get("head_sha")
                .cloned()
                .unwrap_or(Value::Null);
            if !matches!(event_type, "github.rate_limited" | "github.connector_error") {
                let artifact_exists: bool = sqlx::query_scalar("select exists(select 1 from app.artifacts where execution_id=$1 and external_reference_id=$2 and project_id=$3 and workspace_id=$4 and metadata->>'observation_id'=$5)")
                    .bind(execution_id).bind(reference.id).bind(reference.project_id).bind(reference.workspace_id).bind(observation.public_id.to_string()).fetch_one(&mut **tx).await?;
                if !artifact_exists {
                    sqlx::query("insert into app.artifacts(workspace_id,project_id,execution_id,external_reference_id,artifact_type,title,reference,metadata)
                        values($1,$2,$3,$4,'github_observation',$5,$6,$7)")
                        .bind(reference.workspace_id).bind(reference.project_id).bind(execution_id).bind(reference.id)
                        .bind(&reference.display_title).bind(&reference.canonical_url).bind(&payload).execute(&mut **tx).await?;
                }
                let observed_status =
                    external_status(&reference.sync_status, &observation.observed_state);
                sqlx::query("update app.executions set status=$2,result=result || $3,completed_at=case when $2 in ('completed','failed','cancelled') then now() else null end where id=$1 and project_id=$4 and workspace_id=$5")
                    .bind(execution_id).bind(observed_status).bind(json!({"observed_state":observation.observed_state.get("state"),"sync_status":reference.sync_status,"head_sha":payload["head_sha"],"evidence_validation":"human_required"}))
                    .bind(reference.project_id).bind(reference.workspace_id).execute(&mut **tx).await?;
                sqlx::query("update app.tasks set status=$2 where id=(select task_id from app.executions where id=$1) and project_id=$3 and workspace_id=$4")
                    .bind(execution_id).bind(observed_status).bind(reference.project_id).bind(reference.workspace_id).execute(&mut **tx).await?;
            }
        }
        append_event(tx, reference, execution_id, command_id, event_type, payload).await?;
    }
    Ok(())
}

fn external_status(sync_status: &str, observed: &Value) -> &'static str {
    if sync_status == "unavailable" {
        "failed"
    } else if observed.get("merged").and_then(Value::as_bool) == Some(true)
        || observed.get("state").and_then(Value::as_str) == Some("committed")
    {
        "completed"
    } else if observed.get("state").and_then(Value::as_str) == Some("closed") {
        "cancelled"
    } else {
        "running"
    }
}

async fn append_event(
    tx: &mut Transaction<'_, Postgres>,
    reference: &ReferenceRecord,
    execution_id: i64,
    command_id: Uuid,
    event_type: &str,
    payload: Value,
) -> AppResult<()> {
    // execution_ids / evidence_event holds the execution lock for sequence allocation.
    sqlx::query("insert into app.execution_events(workspace_id,project_id,execution_id,event_type,sequence_number,payload)
        select $1,$2,$3,$4,coalesce((select max(sequence_number) from app.execution_events where execution_id=$3),0)+1,$5
        where not exists(select 1 from app.execution_events where execution_id=$3 and event_type=$4 and payload->>'command_id'=$6)")
        .bind(reference.workspace_id).bind(reference.project_id).bind(execution_id).bind(event_type).bind(payload).bind(command_id.to_string()).execute(&mut **tx).await?;
    Ok(())
}

pub(super) async fn validate_artifact(
    tx: &mut Transaction<'_, Postgres>,
    reference: &ReferenceRecord,
    artifact_public_id: Option<Uuid>,
    deliverable_public_id: Uuid,
    head_sha: &str,
) -> AppResult<Option<i64>> {
    let Some(artifact_public_id) = artifact_public_id else {
        if !execution_ids(tx, reference).await?.is_empty() {
            return Err(AppError::Invalid(
                "select the observed artifact from the transmitted ContextPack explicitly".into(),
            ));
        }
        return Ok(None);
    };
    let artifact: Option<(i64, bool)> = sqlx::query_as("select artifact.id,
        coalesce(pack.status='current' and pack.source_graph_version=project.graph_version and artifact.metadata->>'head_sha'=$5
        and artifact.metadata->>'sync_status'='current' and deliverable.source_context_pack_id=pack.id,false)
        from app.artifacts artifact join app.executions execution on execution.id=artifact.execution_id and execution.project_id=artifact.project_id and execution.workspace_id=artifact.workspace_id
        join app.tasks task on task.id=execution.task_id and task.project_id=execution.project_id and task.workspace_id=execution.workspace_id
        join app.context_packs pack on pack.id=task.context_pack_id and pack.project_id=task.project_id and pack.workspace_id=task.workspace_id
        join app.projects project on project.id=pack.project_id and project.workspace_id=pack.workspace_id
        join app.deliverables deliverable on deliverable.public_id=$4 and deliverable.project_id=pack.project_id and deliverable.workspace_id=pack.workspace_id
        where artifact.public_id=$1 and artifact.external_reference_id=$2 and artifact.project_id=$3 and artifact.workspace_id=$6 and execution.executor_key='github-observation'
        for share of project,pack")
        .bind(artifact_public_id).bind(reference.id).bind(reference.project_id).bind(deliverable_public_id).bind(head_sha).bind(reference.workspace_id).fetch_optional(&mut **tx).await?;
    let (artifact_id, current) = artifact.ok_or(AppError::NotFound)?;
    if !current {
        return Err(AppError::Conflict(
            "artifact, delivered plan and current ContextPack must describe the same revision"
                .into(),
        ));
    }
    Ok(Some(artifact_id))
}

pub(super) async fn evidence_event(
    tx: &mut Transaction<'_, Postgres>,
    reference: &ReferenceRecord,
    artifact_id: Option<i64>,
    evidence_id: Uuid,
    command_id: Uuid,
    kind: &str,
) -> AppResult<()> {
    let Some(artifact_id) = artifact_id else {
        return Ok(());
    };
    let execution_id: i64 = sqlx::query_scalar("select execution.id from app.artifacts artifact join app.executions execution on execution.id=artifact.execution_id and execution.project_id=artifact.project_id and execution.workspace_id=artifact.workspace_id where artifact.id=$1 and artifact.external_reference_id=$2 and artifact.project_id=$3 and artifact.workspace_id=$4 for update of execution")
        .bind(artifact_id).bind(reference.id).bind(reference.project_id).bind(reference.workspace_id).fetch_one(&mut **tx).await?;
    append_event(tx, reference, execution_id, command_id, kind, json!({"command_id":command_id,"evidence_id":evidence_id,"external_reference_id":reference.public_id})).await
}

pub(super) async fn load(
    tx: &mut Transaction<'_, Postgres>,
    reference: &ReferenceRecord,
) -> AppResult<Vec<ExternalTrackingView>> {
    let rows: Vec<TrackingRow> = sqlx::query_as("select execution.id execution_id,task.public_id task_public_id,execution.public_id execution_public_id,
        pack.public_id context_pack_public_id,pack.version context_pack_version,pack.content_hash context_pack_hash,
        pack.status='current' and pack.source_graph_version=project.graph_version context_pack_current,
        execution.status,coalesce(execution.result,'{}'::jsonb) observed_result
        from app.edges edge join app.tasks task on task.public_id=edge.target_public_id and task.project_id=edge.project_id and task.workspace_id=edge.workspace_id
        join app.executions execution on execution.task_id=task.id and execution.project_id=task.project_id and execution.workspace_id=task.workspace_id
        join app.context_packs pack on pack.id=task.context_pack_id and pack.project_id=task.project_id and pack.workspace_id=task.workspace_id
        join app.projects project on project.id=pack.project_id and project.workspace_id=pack.workspace_id
        where edge.source_kind='external_reference' and edge.target_kind='task' and edge.edge_type='tracked_by' and edge.status='confirmed'
        and edge.source_public_id=$1 and edge.project_id=$2 and edge.workspace_id=$3 and execution.executor_key='github-observation' order by execution.id")
        .bind(reference.public_id).bind(reference.project_id).bind(reference.workspace_id).fetch_all(&mut **tx).await?;
    let mut result = vec![];
    for row in rows {
        let artifacts = sqlx::query_as("select public_id,reference,metadata,created_at from app.artifacts where execution_id=$1 and external_reference_id=$2 and project_id=$3 and workspace_id=$4 order by created_at,id")
            .bind(row.execution_id).bind(reference.id).bind(reference.project_id).bind(reference.workspace_id).fetch_all(&mut **tx).await?;
        let events = sqlx::query_as("select public_id,event_type,sequence_number,payload,created_at from app.execution_events where execution_id=$1 and project_id=$2 and workspace_id=$3 order by sequence_number")
            .bind(row.execution_id).bind(reference.project_id).bind(reference.workspace_id).fetch_all(&mut **tx).await?;
        result.push(ExternalTrackingView {
            task_public_id: row.task_public_id,
            execution_public_id: row.execution_public_id,
            context_pack_public_id: row.context_pack_public_id,
            context_pack_version: row.context_pack_version,
            context_pack_hash: row.context_pack_hash,
            context_pack_current: row.context_pack_current,
            status: row.status,
            observed_result: row.observed_result,
            artifacts,
            events,
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn green_checks_never_mark_external_work_or_evidence_completed() {
        assert_eq!(
            external_status(
                "current",
                &json!({"state":"open","checks":[{"conclusion":"success"}]})
            ),
            "running"
        );
        assert_eq!(
            external_status("current", &json!({"state":"closed","merged":false})),
            "cancelled"
        );
        assert_eq!(
            external_status("current", &json!({"state":"closed","merged":true})),
            "completed"
        );
        assert_eq!(external_status("unavailable", &json!({})), "failed");
    }
}
