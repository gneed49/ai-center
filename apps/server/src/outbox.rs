//! Transactional `PostgreSQL` outbox primitives.
//!
//! Claiming is intentionally separated from event handling: callers commit the
//! short claim transaction before performing network or model calls, then use a
//! new short transaction to acknowledge success or failure.

use std::cmp::min;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

const MAX_BATCH_SIZE: u32 = 1_000;
const MAX_WORKER_ID_LENGTH: usize = 128;
const MAX_ERROR_CODE_LENGTH: usize = 64;
const MAX_ERROR_MESSAGE_LENGTH: usize = 1_024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxPolicy {
    pub lease_duration: Duration,
    pub max_attempts: u32,
    pub base_backoff: Duration,
    pub max_backoff: Duration,
}

impl Default for OutboxPolicy {
    fn default() -> Self {
        Self {
            lease_duration: Duration::from_secs(60),
            max_attempts: 5,
            base_backoff: Duration::from_secs(30),
            max_backoff: Duration::from_secs(15 * 60),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Outbox {
    policy: OutboxPolicy,
}

#[derive(Debug, Clone, FromRow)]
pub struct ClaimedDomainEvent {
    /// Monotonic database identity used only for stable queue ordering.
    pub sequence_id: i64,
    pub public_id: Uuid,
    pub workspace_id: i64,
    pub project_id: Option<i64>,
    pub event_type: String,
    pub aggregate_kind: String,
    pub aggregate_public_id: Uuid,
    pub payload: Value,
    pub occurred_at: DateTime<Utc>,
    /// Claim generation. It prevents a stale attempt from acknowledging a
    /// later claim made by a worker with the same identifier.
    pub lease_attempt: i32,
    pub locked_until: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureDisposition {
    RetryScheduled { delay: Duration },
    DeadLettered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReclaimSummary {
    pub retry_scheduled: i64,
    pub dead_lettered: i64,
}

#[derive(Debug, Error)]
pub enum OutboxError {
    #[error("invalid outbox policy: {0}")]
    InvalidPolicy(&'static str),
    #[error("worker id must be non-blank and at most {MAX_WORKER_ID_LENGTH} characters")]
    InvalidWorkerId,
    #[error("claim batch size must be between 1 and {MAX_BATCH_SIZE}")]
    InvalidBatchSize,
    #[error("workspace id must be positive")]
    InvalidWorkspaceId,
    #[error("at least one non-blank event type is required")]
    InvalidEventTypes,
    #[error("event {event_public_id} is no longer owned by this lease attempt")]
    LeaseLost { event_public_id: Uuid },
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

impl Outbox {
    /// Builds an outbox using a validated retry and lease policy.
    ///
    /// # Errors
    ///
    /// Returns [`OutboxError::InvalidPolicy`] when a duration or attempt bound
    /// cannot make progress or cannot be represented by `PostgreSQL`.
    pub fn new(policy: OutboxPolicy) -> Result<Self, OutboxError> {
        validate_policy(&policy)?;
        Ok(Self { policy })
    }

    #[must_use]
    pub fn policy(&self) -> &OutboxPolicy {
        &self.policy
    }

    /// Claims available pending events for one explicit workspace and consumer
    /// event-type allowlist without waiting for rows already locked by another
    /// worker. Database RLS still applies through the caller's transaction GUCs.
    /// The caller must commit before processing the events.
    ///
    /// Expired `processing` rows are deliberately not claimed here. Call
    /// [`Self::reclaim_expired_leases`] in the same short transaction first so
    /// crashes follow the same retry/dead-letter policy as explicit failures.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid worker or batch, an unrepresentable
    /// lease duration, or a database failure. The caller owns transaction
    /// commit or rollback.
    pub async fn claim_batch(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        workspace_id: i64,
        event_types: &[String],
        worker_id: &str,
        batch_size: u32,
    ) -> Result<Vec<ClaimedDomainEvent>, OutboxError> {
        validate_workspace_id(workspace_id)?;
        validate_event_types(event_types)?;
        let worker_id = validate_worker_id(worker_id)?;
        if !(1..=MAX_BATCH_SIZE).contains(&batch_size) {
            return Err(OutboxError::InvalidBatchSize);
        }

        let lease_milliseconds = duration_milliseconds(self.policy.lease_duration)?;
        let events = sqlx::query_as::<_, ClaimedDomainEvent>(
            "with candidates as (
               select event.id
               from app.domain_events event
               where event.status = 'pending'
                 and event.workspace_id = $4
                 and event.event_type = any($5)
                 and event.available_at <= clock_timestamp()
               order by event.available_at, event.occurred_at, event.id
               limit $1
               for update skip locked
             ), claimed as (
               update app.domain_events event
               set status = 'processing',
                   attempt_count = event.attempt_count + 1,
                   locked_at = clock_timestamp(),
                   locked_until = clock_timestamp()
                     + make_interval(secs => $3::double precision / 1000.0),
                   locked_by = $2,
                   last_error_code = null,
                   last_error_message = null,
                   processed_at = null,
                   failed_at = null
               from candidates
               where event.id = candidates.id
               returning event.id as sequence_id, event.public_id,
                         event.workspace_id, event.project_id, event.event_type,
                         event.aggregate_kind, event.aggregate_public_id,
                         event.payload, event.occurred_at,
                         event.attempt_count as lease_attempt, event.locked_until
             )
             select * from claimed order by occurred_at, sequence_id",
        )
        .bind(i64::from(batch_size))
        .bind(worker_id)
        .bind(lease_milliseconds)
        .bind(workspace_id)
        .bind(event_types)
        .fetch_all(&mut **tx)
        .await?;

        Ok(events)
    }

    /// Converts expired leases for one explicit workspace and consumer
    /// allowlist into a scheduled retry or a terminal dead letter. Multiple
    /// workers may call this concurrently; `SKIP LOCKED` guarantees that each
    /// expired row is reclaimed by at most one caller.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid batch, an invalid policy value, or a
    /// database failure. The caller owns transaction commit or rollback.
    pub async fn reclaim_expired_leases(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        workspace_id: i64,
        event_types: &[String],
        batch_size: u32,
    ) -> Result<ReclaimSummary, OutboxError> {
        validate_workspace_id(workspace_id)?;
        validate_event_types(event_types)?;
        if !(1..=MAX_BATCH_SIZE).contains(&batch_size) {
            return Err(OutboxError::InvalidBatchSize);
        }

        let base_backoff_milliseconds = duration_milliseconds(self.policy.base_backoff)?;
        let max_backoff_milliseconds = duration_milliseconds(self.policy.max_backoff)?;
        let max_attempts = i32::try_from(self.policy.max_attempts)
            .map_err(|_| OutboxError::InvalidPolicy("max_attempts exceeds PostgreSQL integer"))?;

        let (retry_scheduled, dead_lettered): (i64, i64) = sqlx::query_as(
            "with candidates as (
               select event.id
               from app.domain_events event
               where event.status = 'processing'
                 and event.workspace_id = $5
                 and event.event_type = any($6)
                 and event.locked_until <= clock_timestamp()
               order by event.locked_until, event.occurred_at, event.id
               limit $1
               for update skip locked
             ), reclaimed as (
               update app.domain_events event
               set status = case
                     when event.attempt_count >= $2 then 'dead_letter'
                     else 'pending'
                   end,
                   available_at = case
                     when event.attempt_count >= $2 then event.available_at
                     else clock_timestamp() + make_interval(
                       secs => least(
                         $4::double precision,
                         $3::double precision * power(
                           2::double precision,
                           least(greatest(event.attempt_count - 1, 0), 62)::double precision
                         )
                       ) / 1000.0
                     )
                   end,
                   locked_at = null,
                   locked_until = null,
                   locked_by = null,
                   last_error_code = 'lease_expired',
                   last_error_message = 'worker lease expired before acknowledgement',
                   processed_at = null,
                   failed_at = case
                     when event.attempt_count >= $2 then clock_timestamp()
                     else null
                   end
               from candidates
               where event.id = candidates.id
               returning event.status
             )
             select count(*) filter (where status = 'pending')::bigint,
                    count(*) filter (where status = 'dead_letter')::bigint
             from reclaimed",
        )
        .bind(i64::from(batch_size))
        .bind(max_attempts)
        .bind(base_backoff_milliseconds)
        .bind(max_backoff_milliseconds)
        .bind(workspace_id)
        .bind(event_types)
        .fetch_one(&mut **tx)
        .await?;

        Ok(ReclaimSummary {
            retry_scheduled,
            dead_lettered,
        })
    }

    /// Extends a live lease. The expected attempt is required so an old worker
    /// cannot renew a newer claim that happens to use the same worker id.
    ///
    /// # Errors
    ///
    /// Returns [`OutboxError::LeaseLost`] if the claim is expired, reassigned,
    /// or no longer processing; also returns validation and database errors.
    pub async fn renew_lease(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event_public_id: Uuid,
        worker_id: &str,
        lease_attempt: i32,
    ) -> Result<DateTime<Utc>, OutboxError> {
        let worker_id = validate_worker_id(worker_id)?;
        let lease_milliseconds = duration_milliseconds(self.policy.lease_duration)?;
        let locked_until = sqlx::query_scalar::<_, DateTime<Utc>>(
            "update app.domain_events
             set locked_until = clock_timestamp()
               + make_interval(secs => $4::double precision / 1000.0)
             where public_id = $1
               and status = 'processing'
               and locked_by = $2
               and attempt_count = $3
               and locked_until > clock_timestamp()
             returning locked_until",
        )
        .bind(event_public_id)
        .bind(worker_id)
        .bind(lease_attempt)
        .bind(lease_milliseconds)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(OutboxError::LeaseLost { event_public_id })?;

        Ok(locked_until)
    }

    /// Marks a live claim as successfully processed.
    ///
    /// # Errors
    ///
    /// Returns [`OutboxError::LeaseLost`] if the claim is expired, reassigned,
    /// or no longer processing; also returns validation and database errors.
    pub async fn mark_processed(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event_public_id: Uuid,
        worker_id: &str,
        lease_attempt: i32,
    ) -> Result<(), OutboxError> {
        let worker_id = validate_worker_id(worker_id)?;
        let result = sqlx::query(
            "update app.domain_events
             set status = 'processed',
                 locked_at = null,
                 locked_until = null,
                 locked_by = null,
                 last_error_code = null,
                 last_error_message = null,
                 processed_at = clock_timestamp(),
                 failed_at = null
             where public_id = $1
               and status = 'processing'
               and locked_by = $2
               and attempt_count = $3
               and locked_until > clock_timestamp()",
        )
        .bind(event_public_id)
        .bind(worker_id)
        .bind(lease_attempt)
        .execute(&mut **tx)
        .await?;

        if result.rows_affected() == 1 {
            Ok(())
        } else {
            Err(OutboxError::LeaseLost { event_public_id })
        }
    }

    /// Records a classified, already-sanitized processing failure. Retriable
    /// events are rescheduled with capped exponential backoff; exhausted events
    /// become append-preserving dead letters.
    ///
    /// # Errors
    ///
    /// Returns [`OutboxError::LeaseLost`] if the claim is expired, reassigned,
    /// or no longer processing; also returns validation and database errors.
    pub async fn mark_failed(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event_public_id: Uuid,
        worker_id: &str,
        lease_attempt: i32,
        error_code: &str,
        error_message: &str,
    ) -> Result<FailureDisposition, OutboxError> {
        let worker_id = validate_worker_id(worker_id)?;
        let error_code = bounded_text(error_code, MAX_ERROR_CODE_LENGTH, "unclassified");
        let error_message = bounded_text(
            error_message,
            MAX_ERROR_MESSAGE_LENGTH,
            "event processing failed",
        );
        let attempt_count = u32::try_from(lease_attempt).unwrap_or_default();
        let disposition = transition_after_failure(attempt_count, &self.policy);

        let result = match disposition {
            FailureDisposition::RetryScheduled { delay } => {
                let delay_milliseconds = duration_milliseconds(delay)?;
                sqlx::query(
                    "update app.domain_events
                     set status = 'pending',
                         available_at = clock_timestamp()
                           + make_interval(secs => $6::double precision / 1000.0),
                         locked_at = null,
                         locked_until = null,
                         locked_by = null,
                         last_error_code = $4,
                         last_error_message = $5,
                         processed_at = null,
                         failed_at = null
                     where public_id = $1
                       and status = 'processing'
                       and locked_by = $2
                       and attempt_count = $3
                       and locked_until > clock_timestamp()",
                )
                .bind(event_public_id)
                .bind(worker_id)
                .bind(lease_attempt)
                .bind(&error_code)
                .bind(&error_message)
                .bind(delay_milliseconds)
                .execute(&mut **tx)
                .await?
            }
            FailureDisposition::DeadLettered => {
                sqlx::query(
                    "update app.domain_events
                     set status = 'dead_letter',
                         locked_at = null,
                         locked_until = null,
                         locked_by = null,
                         last_error_code = $4,
                         last_error_message = $5,
                         processed_at = null,
                         failed_at = clock_timestamp()
                     where public_id = $1
                       and status = 'processing'
                       and locked_by = $2
                       and attempt_count = $3
                       and locked_until > clock_timestamp()",
                )
                .bind(event_public_id)
                .bind(worker_id)
                .bind(lease_attempt)
                .bind(&error_code)
                .bind(&error_message)
                .execute(&mut **tx)
                .await?
            }
        };

        if result.rows_affected() == 1 {
            Ok(disposition)
        } else {
            Err(OutboxError::LeaseLost { event_public_id })
        }
    }
}

/// Pure retry decision used by both the worker and unit tests.
#[must_use]
pub fn transition_after_failure(attempt_count: u32, policy: &OutboxPolicy) -> FailureDisposition {
    if attempt_count >= policy.max_attempts {
        FailureDisposition::DeadLettered
    } else {
        FailureDisposition::RetryScheduled {
            delay: retry_backoff(attempt_count, policy.base_backoff, policy.max_backoff),
        }
    }
}

/// Capped exponential backoff. Attempt one receives `base_backoff`, attempt
/// two twice the base, and so on. Attempt zero is treated like attempt one.
#[must_use]
pub fn retry_backoff(attempt_count: u32, base: Duration, maximum: Duration) -> Duration {
    let mut delay = min(base, maximum);
    let doublings = attempt_count.saturating_sub(1);
    for _ in 0..doublings {
        if delay >= maximum {
            return maximum;
        }
        delay = min(delay.checked_mul(2).unwrap_or(maximum), maximum);
    }
    delay
}

fn validate_policy(policy: &OutboxPolicy) -> Result<(), OutboxError> {
    if policy.lease_duration.is_zero() {
        return Err(OutboxError::InvalidPolicy(
            "lease_duration must be greater than zero",
        ));
    }
    if policy.max_attempts == 0 {
        return Err(OutboxError::InvalidPolicy(
            "max_attempts must be greater than zero",
        ));
    }
    if i32::try_from(policy.max_attempts).is_err() {
        return Err(OutboxError::InvalidPolicy(
            "max_attempts exceeds PostgreSQL integer",
        ));
    }
    if policy.base_backoff.is_zero() {
        return Err(OutboxError::InvalidPolicy(
            "base_backoff must be greater than zero",
        ));
    }
    if policy.max_backoff < policy.base_backoff {
        return Err(OutboxError::InvalidPolicy(
            "max_backoff must be greater than or equal to base_backoff",
        ));
    }
    duration_milliseconds(policy.lease_duration)?;
    duration_milliseconds(policy.base_backoff)?;
    duration_milliseconds(policy.max_backoff)?;
    Ok(())
}

fn validate_worker_id(worker_id: &str) -> Result<&str, OutboxError> {
    let worker_id = worker_id.trim();
    if worker_id.is_empty() || worker_id.chars().count() > MAX_WORKER_ID_LENGTH {
        return Err(OutboxError::InvalidWorkerId);
    }
    Ok(worker_id)
}

fn validate_workspace_id(workspace_id: i64) -> Result<(), OutboxError> {
    if workspace_id > 0 {
        Ok(())
    } else {
        Err(OutboxError::InvalidWorkspaceId)
    }
}

fn validate_event_types(event_types: &[String]) -> Result<(), OutboxError> {
    if event_types.is_empty()
        || event_types
            .iter()
            .any(|event_type| event_type.trim().is_empty())
    {
        Err(OutboxError::InvalidEventTypes)
    } else {
        Ok(())
    }
}

fn duration_milliseconds(duration: Duration) -> Result<i64, OutboxError> {
    i64::try_from(duration.as_millis())
        .map_err(|_| OutboxError::InvalidPolicy("duration exceeds PostgreSQL interval range"))
}

fn bounded_text(value: &str, max_characters: usize, fallback: &str) -> String {
    let value = value.trim();
    let value = if value.is_empty() { fallback } else { value };
    value.chars().take(max_characters).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> OutboxPolicy {
        OutboxPolicy {
            lease_duration: Duration::from_secs(60),
            max_attempts: 4,
            base_backoff: Duration::from_secs(5),
            max_backoff: Duration::from_secs(20),
        }
    }

    #[test]
    fn retry_backoff_is_exponential_and_capped() {
        let base = Duration::from_secs(5);
        let maximum = Duration::from_secs(20);

        assert_eq!(retry_backoff(0, base, maximum), Duration::from_secs(5));
        assert_eq!(retry_backoff(1, base, maximum), Duration::from_secs(5));
        assert_eq!(retry_backoff(2, base, maximum), Duration::from_secs(10));
        assert_eq!(retry_backoff(3, base, maximum), Duration::from_secs(20));
        assert_eq!(retry_backoff(50, base, maximum), Duration::from_secs(20));
    }

    #[test]
    fn failure_transition_retries_until_max_attempts() {
        let policy = policy();

        assert_eq!(
            transition_after_failure(1, &policy),
            FailureDisposition::RetryScheduled {
                delay: Duration::from_secs(5)
            }
        );
        assert_eq!(
            transition_after_failure(3, &policy),
            FailureDisposition::RetryScheduled {
                delay: Duration::from_secs(20)
            }
        );
        assert_eq!(
            transition_after_failure(4, &policy),
            FailureDisposition::DeadLettered
        );
        assert_eq!(
            transition_after_failure(5, &policy),
            FailureDisposition::DeadLettered
        );
    }

    #[test]
    fn policy_rejects_unbounded_or_non_progressing_values() {
        let mut invalid = policy();
        invalid.lease_duration = Duration::ZERO;
        assert!(matches!(
            Outbox::new(invalid),
            Err(OutboxError::InvalidPolicy(_))
        ));

        let mut invalid = policy();
        invalid.max_attempts = 0;
        assert!(matches!(
            Outbox::new(invalid),
            Err(OutboxError::InvalidPolicy(_))
        ));

        let mut invalid = policy();
        invalid.max_backoff = Duration::from_secs(1);
        assert!(matches!(
            Outbox::new(invalid),
            Err(OutboxError::InvalidPolicy(_))
        ));
    }

    #[test]
    fn bounded_error_text_is_trimmed_and_never_blank() {
        assert_eq!(bounded_text("  transient  ", 64, "fallback"), "transient");
        assert_eq!(bounded_text("   ", 64, "fallback"), "fallback");
        assert_eq!(bounded_text("abcdef", 3, "fallback"), "abc");
    }

    #[test]
    fn workspace_and_event_filters_fail_closed_before_claiming() {
        assert!(matches!(
            validate_workspace_id(0),
            Err(OutboxError::InvalidWorkspaceId)
        ));
        assert!(validate_workspace_id(42).is_ok());
        assert!(matches!(
            validate_event_types(&[]),
            Err(OutboxError::InvalidEventTypes)
        ));
        assert!(matches!(
            validate_event_types(&["  ".into()]),
            Err(OutboxError::InvalidEventTypes)
        ));
        assert!(
            validate_event_types(&["knowledge.committed".into(), "knowledge.revised".into()])
                .is_ok()
        );
    }
}
