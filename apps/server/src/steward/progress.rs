//! Durable, fenced source frontier. A receipt is a search boundary, not a
//! guarantee that every semantic relationship has been found.
use super::company::{MAX_NEIGHBORS, MAX_SOURCES, Source};
use crate::{
    error::{AppError, AppResult},
    service::AppState,
};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub(super) const CALLS_PER_HOUR: i64 = 6;
#[derive(Clone, Debug)]
pub(super) struct Step {
    pub lease: Uuid,
    pub anchor: Option<Source>,
    pub omitted_neighbors: i64,
    pub has_more: bool,
    pub blocked: bool,
}
impl Step {
    #[cfg(test)]
    pub fn test_empty() -> Self {
        Self {
            lease: Uuid::nil(),
            anchor: None,
            omitted_neighbors: 0,
            has_more: false,
            blocked: false,
        }
    }
}

pub(super) async fn claim(tx: &mut Transaction<'_, Postgres>) -> AppResult<Uuid> {
    sqlx::query("insert into app.steward_scan_progress(workspace_id) values(app.current_workspace_id()) on conflict(workspace_id) do nothing").execute(&mut **tx).await?;
    let token = Uuid::new_v4();
    let claimed=sqlx::query("update app.steward_scan_progress set status='running',lease_token=$1,lease_until=clock_timestamp()+interval '11 minutes',last_error_code=null,updated_at=clock_timestamp() where workspace_id=app.current_workspace_id() and (lease_token is null or lease_until<clock_timestamp())")
        .bind(token).execute(&mut **tx).await?;
    if claimed.rows_affected() != 1 {
        return Err(AppError::Capacity {
            retry_after_seconds: 60,
        });
    }
    Ok(token)
}

pub(super) async fn admit(tx: &mut Transaction<'_, Postgres>) -> AppResult<()> {
    // The company lease serializes this check and subsequent model-run insert.
    // All attempts, including failures, count. Existing actor/company AI quotas
    // are still enforced independently by the provider wrapper.
    let count:i64=sqlx::query_scalar("select count(*) from app.model_runs where workspace_id=app.current_workspace_id() and operation='assess_contradiction' and created_at>clock_timestamp()-interval '1 hour'")
        .fetch_one(&mut **tx).await?;
    if count >= CALLS_PER_HOUR {
        return Err(AppError::Capacity {
            retry_after_seconds: 300,
        });
    }
    Ok(())
}

pub(super) async fn lock_current(tx: &mut Transaction<'_, Postgres>, step: &Step) -> AppResult<()> {
    let valid:bool=sqlx::query_scalar("select coalesce(lease_token=$1 and lease_until>clock_timestamp(),false) from app.steward_scan_progress where workspace_id=app.current_workspace_id() for update")
        .bind(step.lease).fetch_optional(&mut **tx).await?.unwrap_or(false);
    if !valid {
        return Err(AppError::Conflict(
            "Le bail de cette analyse a expiré ; sa reprise doit être vérifiée.".into(),
        ));
    }
    Ok(())
}

pub(super) async fn finish(
    tx: &mut Transaction<'_, Postgres>,
    step: &Step,
    examined: usize,
) -> AppResult<()> {
    lock_current(tx, step).await?;
    if let Some(anchor) = &step.anchor {
        sqlx::query("insert into app.steward_scan_sources(workspace_id,project_id,source_public_id,source_kind,examined_pairs,omitted_neighbors) values(app.current_workspace_id(),$1,$2,$3,$4,$5) on conflict(workspace_id,source_public_id) do nothing")
            .bind(anchor.source_project_id).bind(anchor.source_public_id).bind(&anchor.source_kind)
            .bind(i32::try_from(examined).unwrap_or(i32::MAX)).bind(step.omitted_neighbors).execute(&mut **tx).await?;
    }
    let status = if step.blocked {
        "blocked"
    } else if step.has_more {
        "pending"
    } else {
        "idle"
    };
    let finished = sqlx::query("update app.steward_scan_progress set status=$2,lease_token=null,lease_until=null,last_error_code=$3,updated_at=clock_timestamp() where workspace_id=app.current_workspace_id() and lease_token=$1 and lease_until>clock_timestamp()")
        .bind(step.lease).bind(status).bind(step.blocked.then_some("source_catalog_limit")).execute(&mut **tx).await?;
    if finished.rows_affected() != 1 {
        return Err(AppError::Conflict(
            "Le bail de cette analyse a expiré ; aucun résultat n’a été enregistré.".into(),
        ));
    }
    if step.has_more && !step.blocked {
        sqlx::query("insert into app.domain_events(workspace_id,project_id,event_type,aggregate_kind,aggregate_public_id,payload,requested_by_actor_id,available_at) select app.current_workspace_id(),p.id,'steward.continue','project',p.public_id,'{}'::jsonb,app.current_actor_id(),clock_timestamp()+interval '60 seconds' from app.projects p where p.workspace_id=app.current_workspace_id() and p.scope_kind='company' and p.status='active' and not exists(select 1 from app.domain_events e where e.workspace_id=p.workspace_id and e.event_type='steward.continue' and e.status='pending' and e.requested_by_actor_id=app.current_actor_id())")
            .execute(&mut **tx).await?;
    }
    Ok(())
}

pub(super) async fn release(state: &AppState, step: &Step, code: &str) -> AppResult<()> {
    let mut tx = state.begin_request().await?;
    sqlx::query("update app.steward_scan_progress set status='pending',lease_token=null,lease_until=null,last_error_code=$2,updated_at=clock_timestamp() where workspace_id=app.current_workspace_id() and lease_token=$1")
        .bind(step.lease).bind(code).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

/// Read-only company coverage; never returns the lease capability or content.
pub async fn status(state: &AppState) -> AppResult<Value> {
    let mut tx = state.begin_request().await?;
    let company:Option<i64>=sqlx::query_scalar("select id from app.projects where workspace_id=app.current_workspace_id() and scope_kind='company' and status='active'").fetch_optional(&mut *tx).await?;
    let (mut available, mut pending): (i64, i64) = (0, 0);
    if let Some(project) = company {
        (available,pending)=sqlx::query_as(&format!("select count(*),count(*) filter(where not exists(select 1 from app.steward_scan_sources s where s.workspace_id=app.current_workspace_id() and s.source_public_id=c.source_public_id)) from ({}) c",include_str!("company_sources.sql")))
            .bind(project).fetch_one(&mut *tx).await?;
    }
    let progress:Option<Value>=sqlx::query_scalar("select jsonb_build_object('status',status,'last_error_code',last_error_code,'updated_at',updated_at,'lease_expired',lease_until<clock_timestamp()) from app.steward_scan_progress where workspace_id=app.current_workspace_id()")
        .fetch_optional(&mut *tx).await?;
    let (examined,omitted):(i64,i64)=sqlx::query_as("select coalesce(sum(examined_pairs),0)::bigint,coalesce(sum(omitted_neighbors),0)::bigint from app.steward_scan_sources where workspace_id=app.current_workspace_id()")
        .fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(
        json!({"progress":progress,"available_sources":available,"pending_sources":pending,"examined_pairs":examined,"omitted_neighbors":omitted,
        "exhaustive":false,"max_sources":MAX_SOURCES,"max_neighbors_per_source":MAX_NEIGHBORS,"max_provider_calls_per_hour":CALLS_PER_HOUR,
        "coverage_notice":"Analyse des versions nouvelles par voisinage pertinent. Les voisins omis, les sources non lues et les formulations sans termes communs ne sont pas couverts. Un parcours terminé ne certifie pas l’absence de contradictions."}),
    )
}
