//! Persisted per-company admission limits; replicas share the same quota lock.
use crate::{
    artifacts::editor,
    error::{AppError, AppResult},
    service::AppState,
};
use serde::Serialize;
use serde_json::json;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub const MAX_PENDING: i64 = 25;
pub const MAX_PUBLICATIONS_HOUR: i64 = 100;
pub const MAX_READS_HOUR: i64 = 120;
#[derive(Serialize)]
pub struct Limits {
    pub max_pending_publications: i64,
    pub max_publications_per_hour: i64,
    pub max_remote_reads_per_hour: i64,
    pub create_attempts_per_job: i32,
    pub request_timeout_seconds: i32,
}
pub fn limits() -> AppResult<Limits> {
    Ok(Limits {
        max_pending_publications: configured("AI_CENTER_WORK_TOOLS_MAX_PENDING", MAX_PENDING, 100)?,
        max_publications_per_hour: configured(
            "AI_CENTER_WORK_TOOLS_PUBLICATIONS_PER_HOUR",
            MAX_PUBLICATIONS_HOUR,
            1000,
        )?,
        max_remote_reads_per_hour: configured(
            "AI_CENTER_WORK_TOOLS_READS_PER_HOUR",
            MAX_READS_HOUR,
            1200,
        )?,
        create_attempts_per_job: 1,
        request_timeout_seconds: 30,
    })
}
fn configured(name: &str, default: i64, max: i64) -> AppResult<i64> {
    match std::env::var(name) {
        Ok(raw) => parse_limit(&raw, max),
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(_) => Err(AppError::Internal("Work tool limits are invalid".into())),
    }
}
fn parse_limit(raw: &str, max: i64) -> AppResult<i64> {
    raw.parse::<i64>()
        .ok()
        .filter(|value| (1..=max).contains(value))
        .ok_or_else(|| AppError::Internal("Work tool limits are invalid".into()))
}
#[derive(Serialize, sqlx::FromRow)]
pub struct Usage {
    pub queued: i64,
    pub processing: i64,
    pub needs_review: i64,
    pub publications_last_hour: i64,
    pub remote_read_operations_last_hour: i64,
    /// No provider billing receipt is available for these API operations.
    pub known_cost_usd: Option<f64>,
}
pub(super) async fn usage(tx: &mut Transaction<'_, Postgres>) -> AppResult<Usage> {
    Ok(sqlx::query_as("select count(*) filter(where status='queued') as queued,count(*) filter(where status='processing') as processing,count(*) filter(where status='needs_review') as needs_review,count(*) filter(where created_at>now()-interval '1 hour') as publications_last_hour,(select count(*) from app.audit_events where workspace_id=app.current_workspace_id() and action='work_tool.remote_read.admitted' and occurred_at>now()-interval '1 hour') as remote_read_operations_last_hour,null::double precision as known_cost_usd from app.publication_jobs where workspace_id=app.current_workspace_id()")
        .fetch_one(&mut **tx).await?)
}
async fn quota_lock(tx: &mut Transaction<'_, Postgres>) -> AppResult<()> {
    sqlx::query("select pg_advisory_xact_lock(hashtextextended(app.current_workspace_id()::text||':work-tool-quota',0))")
        .execute(&mut **tx).await?;
    Ok(())
}
pub(super) async fn admit_publication(tx: &mut Transaction<'_, Postgres>) -> AppResult<()> {
    admit_publications(tx, 1).await
}
pub(super) async fn admit_publications(
    tx: &mut Transaction<'_, Postgres>,
    count: i64,
) -> AppResult<()> {
    if count == 0 {
        return Ok(());
    }
    if !(1..=30).contains(&count) {
        return Err(AppError::Invalid("Invalid publication count".into()));
    }
    quota_lock(tx).await?;
    let usage = usage(tx).await?;
    let limits = limits()?;
    if usage.publications_last_hour + count > limits.max_publications_per_hour {
        return Err(AppError::Capacity {
            retry_after_seconds: 3600,
        });
    }
    if usage.queued + usage.processing + count > limits.max_pending_publications {
        return Err(AppError::Capacity {
            retry_after_seconds: 60,
        });
    }
    Ok(())
}
pub(super) async fn admit_read(state: &AppState, object: Uuid) -> AppResult<()> {
    editor(state)?;
    let mut tx = state.begin_request().await?;
    quota_lock(&mut tx).await?;
    if usage(&mut tx).await?.remote_read_operations_last_hour >= limits()?.max_remote_reads_per_hour
    {
        return Err(AppError::Capacity {
            retry_after_seconds: 3600,
        });
    }
    super::audit(
        &mut tx,
        state,
        "work_tool.remote_read.admitted",
        object,
        json!({}),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quota_configuration_never_disables_a_guard_with_zero_or_invalid_values() {
        assert_eq!(parse_limit("25", 100).expect("valid"), 25);
        for raw in ["0", "-1", "101", "unknown", ""] {
            assert!(parse_limit(raw, 100).is_err());
        }
        let error = AppError::Capacity {
            retry_after_seconds: 60,
        };
        assert_eq!(
            error.status_code(),
            axum::http::StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(error.public_code(), "capacity_exceeded");
    }
}
