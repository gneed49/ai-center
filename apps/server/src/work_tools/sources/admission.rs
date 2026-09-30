//! Company-wide network accounting without access to another actor's command.
use super::models::{ExistingReadErrorCode, ExistingReadFailure};
use crate::{
    error::{AppError, AppResult},
    idempotency::IdempotencyLease,
    service::AppState,
    work_tools::{audit, reliability},
};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub(super) fn refusal(
    code: &'static str,
    message: &'static str,
    retry_after: Option<DateTime<Utc>>,
    status: StatusCode,
) -> AppError {
    AppError::ToolSource {
        code,
        message,
        retry_after,
        status,
        retryable: status == StatusCode::TOO_MANY_REQUESTS,
    }
}
pub(super) fn remote_error(failure: &ExistingReadFailure) -> AppError {
    let status = match failure.code {
        ExistingReadErrorCode::RemoteRateLimit => StatusCode::TOO_MANY_REQUESTS,
        ExistingReadErrorCode::RemoteTimeout => StatusCode::GATEWAY_TIMEOUT,
        ExistingReadErrorCode::RemoteUnavailable | ExistingReadErrorCode::RemoteTransport => {
            StatusCode::BAD_GATEWAY
        }
        _ => StatusCode::UNPROCESSABLE_ENTITY,
    };
    AppError::ToolSource {
        code: failure.code.as_str(),
        message: "La lecture distante n’a pas abouti. Vérifiez l’accès à la source avant de réessayer.",
        status,
        retry_after: failure
            .retry_after_seconds
            .map(|n| Utc::now() + chrono::Duration::seconds(i64::from(n.clamp(1, 86_400)))),
        retryable: failure.retryable,
    }
}
pub(super) async fn enabled(tx: &mut Transaction<'_, Postgres>) -> AppResult<()> {
    sqlx::query("select pg_advisory_xact_lock_shared(hashtextextended('ai-automation:'||app.current_workspace_id()::text,0))").execute(&mut **tx).await?;
    let enabled:bool=sqlx::query_scalar("select coalesce((select enabled from app.workspace_automation_controls where workspace_id=app.current_workspace_id()),true)").fetch_one(&mut **tx).await?;
    if !enabled {
        return Err(AppError::Conflict(
            "Les lectures externes sont suspendues pour cette société.".into(),
        ));
    }
    Ok(())
}
pub(super) struct Attempt {
    pub id: Uuid,
    pub admitted_at: DateTime<Utc>,
    pub connection: Uuid,
    pub command: Uuid,
    pub generation: Uuid,
}
pub(super) async fn admit(
    tx: &mut Transaction<'_, Postgres>,
    state: &AppState,
    connection: Uuid,
    fingerprint: &str,
    last_attempt: Option<DateTime<Utc>>,
    lease: &IdempotencyLease,
) -> AppResult<Attempt> {
    reliability::quota_lock(tx).await?;
    enabled(tx).await?;
    let now: DateTime<Utc> = sqlx::query_scalar("select clock_timestamp()")
        .fetch_one(&mut **tx)
        .await?;
    if let Some(retry) = reliability::read_retry_after(tx, connection).await? {
        return Err(refusal(
            "remote_rate_limit",
            "Cet outil demande de patienter avant une nouvelle lecture.",
            Some(retry),
            StatusCode::TOO_MANY_REQUESTS,
        ));
    }
    let previous: Option<DateTime<Utc>> = if last_attempt.is_none() {
        sqlx::query_scalar("select max(occurred_at) from app.audit_events where workspace_id=app.current_workspace_id() and action='work_tool.remote_read.admitted' and occurred_at>clock_timestamp()-interval '60 seconds' and after_state->>'input_fingerprint'=$1").bind(fingerprint).fetch_one(&mut **tx).await?
    } else {
        None
    };
    if let Some(last) = last_attempt
        .into_iter()
        .chain(previous)
        .max()
        .filter(|v| *v + chrono::Duration::seconds(60) > now)
    {
        return Err(refusal(
            "source_refresh_too_soon",
            "Cette source vient d’être lue. Patientez avant une nouvelle vérification.",
            Some(last + chrono::Duration::seconds(60)),
            StatusCode::TOO_MANY_REQUESTS,
        ));
    }
    let (count,oldest):(i64,Option<DateTime<Utc>>)=sqlx::query_as("select count(*),min(occurred_at) from app.audit_events where workspace_id=app.current_workspace_id() and action='work_tool.remote_read.admitted' and occurred_at>clock_timestamp()-interval '1 hour'").fetch_one(&mut **tx).await?;
    if count >= reliability::limits()?.max_remote_reads_per_hour {
        return Err(refusal(
            "source_read_quota_exceeded",
            "Le budget horaire de lecture de la société est atteint.",
            Some(oldest.unwrap_or(now) + chrono::Duration::hours(1)),
            StatusCode::TOO_MANY_REQUESTS,
        ));
    }
    // Slots contain server generated RFC3339 dates only; no credential or source text.
    let slots:Vec<String>=sqlx::query_scalar("select a.after_state->>'slot_expires_at' from app.audit_events a where a.workspace_id=app.current_workspace_id() and a.action='work_tool.remote_read.admitted' and a.occurred_at>clock_timestamp()-interval '60 seconds' and jsonb_typeof(a.after_state->'slot_expires_at')='string' and not exists(select 1 from app.audit_events f where f.workspace_id=a.workspace_id and f.action='work_tool.remote_read.finished' and f.occurred_at>=a.occurred_at and f.after_state->>'attempt_id'=a.after_state->>'attempt_id' and f.after_state->>'command_public_id'=a.after_state->>'command_public_id' and f.after_state->>'lease_generation'=a.after_state->>'lease_generation')").fetch_all(&mut **tx).await?;
    let slots: Vec<_> = slots
        .iter()
        .filter_map(|v| DateTime::parse_from_rfc3339(v).ok())
        .map(|v| v.with_timezone(&Utc))
        .filter(|v| *v > now)
        .collect();
    if slots.len() >= 3 {
        return Err(refusal(
            "source_read_in_progress",
            "Trois lectures sont déjà en cours pour cette société.",
            slots.into_iter().min(),
            StatusCode::TOO_MANY_REQUESTS,
        ));
    }
    // Last lock in the protocol: no subsequent business lock after this receipt.
    if !crate::idempotency::renew(tx, lease).await? {
        return Err(AppError::Conflict(
            "Cette commande n’est plus active.".into(),
        ));
    }
    let attempt = Attempt {
        id: Uuid::new_v4(),
        admitted_at: now,
        connection,
        command: lease.record_public_id,
        generation: lease.generation,
    };
    audit(tx,state,"work_tool.remote_read.admitted",connection,json!({"connection_id":connection,"input_fingerprint":fingerprint,"command_public_id":attempt.command,"lease_generation":attempt.generation,"attempt_id":attempt.id,"slot_expires_at":now+chrono::Duration::seconds(60)})).await?;
    Ok(attempt)
}
pub(super) async fn finished(
    state: &AppState,
    attempt: &Attempt,
    failure: Option<&ExistingReadFailure>,
) -> AppResult<()> {
    let mut tx = state.begin_request().await?;
    audit(&mut tx,state,"work_tool.remote_read.finished",attempt.connection,json!({"command_public_id":attempt.command,"lease_generation":attempt.generation,"attempt_id":attempt.id})).await?;
    if let Some(failure) = failure.filter(|f| f.code == ExistingReadErrorCode::RemoteRateLimit) {
        audit(&mut tx,state,"work_tool.remote_read.rate_limited",attempt.connection,json!({"connection_id":attempt.connection,"retry_after":Utc::now()+chrono::Duration::seconds(i64::from(failure.retry_after_seconds.unwrap_or(60).clamp(1,86_400)))})).await?;
    }
    tx.commit().await?;
    Ok(())
}
