use serde_json::json;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::{
    audit, complete, editor,
    models::{ARTIFACT_TYPES, Destination, Destinations, ResetDestination, SetDestination},
    project_id, validation,
};
use crate::{
    error::{AppError, AppResult},
    idempotency::IdempotencyLease,
    service::AppState,
};

#[derive(sqlx::FromRow)]
struct Setting {
    project_id: Option<i64>,
    artifact_type: String,
    provider: String,
    target_id: Option<String>,
    label: String,
    revision: i32,
    enabled: bool,
}

async fn read(
    tx: &mut Transaction<'_, Postgres>,
    project: Option<(i64, Uuid)>,
) -> AppResult<Destinations> {
    let rows: Vec<Setting> = sqlx::query_as(
        "select project_id,artifact_type,provider,target_id,label,revision,enabled
        from app.artifact_destination_settings where workspace_id=app.current_workspace_id()
        and (project_id is null or project_id=$1)",
    )
    .bind(project.map(|p| p.0))
    .fetch_all(&mut **tx)
    .await?;
    let items = ARTIFACT_TYPES
        .into_iter()
        .map(|kind| {
            let own = rows
                .iter()
                .find(|r| r.artifact_type == kind && r.project_id == project.map(|p| p.0));
            let inherited = rows
                .iter()
                .find(|r| r.artifact_type == kind && r.project_id.is_none() && r.enabled);
            let active = own.filter(|r| r.enabled).or(inherited);
            Destination {
                artifact_type: kind.into(),
                provider: active.map_or("internal", |r| r.provider.as_str()).into(),
                target_id: active.and_then(|r| r.target_id.clone()),
                label: active.map_or("", |r| r.label.as_str()).into(),
                origin: active
                    .map_or("default", |r| {
                        if r.project_id.is_some() {
                            "project"
                        } else {
                            "company"
                        }
                    })
                    .into(),
                // Revision of the edited scope, including its reset tombstone.
                revision: own.map_or(0, |r| r.revision),
                project_id: project.map(|p| p.1),
            }
        })
        .collect();
    Ok(Destinations { items })
}

/// Resolves project overrides, company defaults and internal fallback.
///
/// # Errors
/// Returns an inaccessible project or database failure.
pub async fn destinations(state: &AppState, project: Option<Uuid>) -> AppResult<Destinations> {
    let mut tx = state.begin_request().await?;
    let project = match project {
        Some(id) => Some((project_id(&mut tx, id).await?, id)),
        None => None,
    };
    let result = read(&mut tx, project).await?;
    tx.commit().await?;
    Ok(result)
}

fn authorize(state: &AppState, project: Option<Uuid>) -> AppResult<()> {
    editor(state)?;
    if project.is_none() && state.workspace_role != "owner" {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

async fn lock_revision(
    tx: &mut Transaction<'_, Postgres>,
    project: Option<i64>,
    kind: &str,
    expected: i32,
) -> AppResult<()> {
    // Serializes first insertion as well as update/reset without broad table locks.
    sqlx::query("select pg_advisory_xact_lock(hashtextextended(app.current_workspace_id()::text||':artifact-destination:'||coalesce($1::text,'company')||':'||$2,0))")
        .bind(project.map(|id|id.to_string())).bind(kind).execute(&mut **tx).await?;
    let current:Option<i32>=sqlx::query_scalar("select revision from app.artifact_destination_settings
        where workspace_id=app.current_workspace_id() and project_id is not distinct from $1 and artifact_type=$2")
        .bind(project).bind(kind).fetch_optional(&mut **tx).await?;
    if current.unwrap_or(0) != expected {
        return Err(AppError::Conflict(
            "Destination settings changed. Reload before saving.".into(),
        ));
    }
    Ok(())
}

/// Saves a scoped destination without contacting its provider or publishing.
///
/// # Errors
/// Rejects unauthorized roles, invalid targets, stale revisions and database failures.
pub async fn set_destination(
    state: &AppState,
    project: Option<Uuid>,
    input: SetDestination,
    lease: Option<&IdempotencyLease>,
) -> AppResult<Destinations> {
    authorize(state, project)?;
    validation::destination(&input)?;
    let mut tx = state.begin_request().await?;
    let scope = match project {
        Some(id) => Some((project_id(&mut tx, id).await?, id)),
        None => None,
    };
    if let Some((project, _)) = scope {
        crate::company::data::require_active(&mut tx, project).await?;
    }
    lock_revision(
        &mut tx,
        scope.map(|p| p.0),
        &input.artifact_type,
        input.expected_revision,
    )
    .await?;
    if input.expected_revision == 0 {
        sqlx::query("insert into app.artifact_destination_settings(workspace_id,project_id,artifact_type,provider,target_id,label,updated_by_actor_id)
            values(app.current_workspace_id(),$1,$2,$3,$4,$5,$6)")
            .bind(scope.map(|p|p.0)).bind(&input.artifact_type).bind(&input.provider).bind(&input.target_id)
            .bind(input.label.trim()).bind(state.actor_id).execute(&mut *tx).await?;
    } else {
        sqlx::query("update app.artifact_destination_settings set provider=$3,target_id=$4,label=$5,revision=revision+1,enabled=true,updated_by_actor_id=$6,updated_at=now()
            where workspace_id=app.current_workspace_id() and project_id is not distinct from $1 and artifact_type=$2")
            .bind(scope.map(|p|p.0)).bind(&input.artifact_type).bind(&input.provider).bind(&input.target_id)
            .bind(input.label.trim()).bind(state.actor_id).execute(&mut *tx).await?;
    }
    audit(&mut tx,state.actor_id,scope.map(|p|p.0),"artifact.destination.updated",project.unwrap_or(state.workspace_id),
        json!({"artifact_type":input.artifact_type,"provider":input.provider,"revision":input.expected_revision+1})).await?;
    let output = read(&mut tx, scope).await?;
    complete(&mut tx, lease, &output).await?;
    tx.commit().await?;
    Ok(output)
}

/// Restores inheritance while retaining the scope's monotonically increasing revision.
///
/// # Errors
/// Rejects unauthorized roles, absent/stale revisions and database failures.
pub async fn reset_destination(
    state: &AppState,
    project: Option<Uuid>,
    input: ResetDestination,
    lease: Option<&IdempotencyLease>,
) -> AppResult<Destinations> {
    authorize(state, project)?;
    validation::artifact_type(&input.artifact_type)?;
    if input.expected_revision <= 0 {
        return Err(AppError::Conflict(
            "No existing destination revision to reset".into(),
        ));
    }
    let mut tx = state.begin_request().await?;
    let scope = match project {
        Some(id) => Some((project_id(&mut tx, id).await?, id)),
        None => None,
    };
    if let Some((project, _)) = scope {
        crate::company::data::require_active(&mut tx, project).await?;
    }
    lock_revision(
        &mut tx,
        scope.map(|p| p.0),
        &input.artifact_type,
        input.expected_revision,
    )
    .await?;
    sqlx::query("update app.artifact_destination_settings set provider='internal',target_id=null,label='',enabled=false,revision=revision+1,updated_by_actor_id=$3,updated_at=now()
        where workspace_id=app.current_workspace_id() and project_id is not distinct from $1 and artifact_type=$2")
        .bind(scope.map(|p|p.0)).bind(&input.artifact_type).bind(state.actor_id).execute(&mut *tx).await?;
    audit(
        &mut tx,
        state.actor_id,
        scope.map(|p| p.0),
        "artifact.destination.reset",
        project.unwrap_or(state.workspace_id),
        json!({"artifact_type":input.artifact_type,"revision":input.expected_revision+1}),
    )
    .await?;
    let output = read(&mut tx, scope).await?;
    complete(&mut tx, lease, &output).await?;
    tx.commit().await?;
    Ok(output)
}
