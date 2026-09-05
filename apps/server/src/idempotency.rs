//! Durable idempotency protocol for externally retried mutations.
//!
//! The caller owns the surrounding short database transaction. This is
//! intentional: request-scoped `PostgreSQL` settings (actor, workspace and
//! role) must be installed on that same transaction before these functions
//! run, so forced RLS remains effective.

use std::{collections::BTreeMap, future::Future, time::Duration};

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

/// Duration for which one worker owns a processing record before a retry may
/// reclaim it.
pub const LEASE_DURATION_SECONDS: i32 = 30;

/// Minimum duration during which a completed response is replayable.
pub const RETENTION_SECONDS: i32 = 24 * 60 * 60;

const MAX_OPERATION_KEY_BYTES: usize = 128;
const MAX_IDEMPOTENCY_KEY_BYTES: usize = 255;
const MAX_ERROR_CODE_BYTES: usize = 128;

/// Scope and fingerprint supplied when a mutating operation begins.
#[derive(Debug, Clone, Copy)]
pub struct BeginRequest<'a> {
    pub workspace_id: i64,
    pub project_id: Option<i64>,
    pub actor_id: Uuid,
    pub operation_key: &'a str,
    pub idempotency_key: &'a str,
    pub request_hash: &'a str,
}

/// Proof that the caller currently owns a durable idempotency record.
///
/// `generation` fences each claimant independently of the renewable deadline.
/// Reclaiming changes it atomically, so a stale worker cannot finalize.
#[derive(Debug, Clone, Serialize)]
pub struct IdempotencyLease {
    pub record_public_id: Uuid,
    pub workspace_id: i64,
    pub project_id: Option<i64>,
    pub actor_id: Uuid,
    pub operation_key: String,
    pub idempotency_key: String,
    pub request_hash: String,
    pub generation: Uuid,
    pub locked_until: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// A response stored for exact replay after the original HTTP response is
/// lost or the client retries.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StoredResponse {
    pub status_code: u16,
    pub body: Value,
    pub failed: bool,
    /// Whether the exact same key and request may atomically resume this
    /// failed command. Completed responses and permanent failures are always
    /// replayed instead.
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

/// Explicit durability policy for a failed command.
///
/// Callers must opt in to a retry. A gateway status alone is not sufficient:
/// configuration, authentication or provider-contract failures may also be
/// surfaced through a gateway response but must remain replayable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureDisposition {
    Permanent,
    Retryable,
}

/// Result of trying to begin a durable mutation.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum BeginOutcome {
    /// This caller owns the operation and may perform it once.
    New {
        lease: IdempotencyLease,
        /// `true` when an abandoned processing lease or retryable failure was
        /// reclaimed.
        reclaimed: bool,
        /// `true` only for an atomic `failed -> processing` transition after
        /// an explicitly retryable 502/503/gateway-timeout failure.
        retrying_transient_failure: bool,
    },
    /// Another worker still owns the same request.
    InProgress {
        record_public_id: Uuid,
        locked_until: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    },
    /// The original result must be returned without running the mutation.
    Replay {
        record_public_id: Uuid,
        response: StoredResponse,
        expires_at: DateTime<Utc>,
    },
}

#[derive(Debug, sqlx::FromRow)]
struct IdempotencyRecord {
    id: i64,
    public_id: Uuid,
    workspace_id: i64,
    project_id: Option<i64>,
    actor_id: Uuid,
    operation_key: String,
    idempotency_key: String,
    request_hash: String,
    status: String,
    response_status: Option<i16>,
    response_body: Option<Value>,
    error_code: Option<String>,
    lease_generation: Uuid,
    locked_until: Option<DateTime<Utc>>,
    expires_at: DateTime<Utc>,
    expired: bool,
    reclaimable: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct TransactionScope {
    actor_id: Option<Uuid>,
    workspace_id: Option<i64>,
    workspace_role: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExistingDecision {
    ResetExpired,
    ScopeConflict,
    HashConflict,
    Reclaim,
    RetryFailed,
    InProgress,
    Replay,
}

/// Returns a SHA-256 digest of canonical JSON.
///
/// Object keys are recursively sorted, so semantically identical JSON bodies
/// do not become different requests merely because fields arrived in a
/// different order.
#[must_use]
pub fn hash_json(value: &Value) -> String {
    let canonical = canonicalize_json(value);
    let serialized = canonical.to_string();
    format!("{:x}", Sha256::digest(serialized.as_bytes()))
}

/// Serializes a typed request and returns its canonical request hash.
///
/// # Errors
///
/// Returns an internal error when the typed request cannot be represented as
/// JSON.
pub fn hash_request<T: Serialize + ?Sized>(request: &T) -> AppResult<String> {
    let value = serde_json::to_value(request).map_err(|error| {
        AppError::Internal(format!("failed to fingerprint idempotent request: {error}"))
    })?;
    Ok(hash_json(&value))
}

/// Claims an idempotency record, reports an active concurrent operation, or
/// returns the response recorded by the first successful claimant.
///
/// The surrounding transaction must already contain the request-scoped RLS
/// settings. Commit it before doing network or model work.
///
/// # Errors
///
/// Returns `Invalid` for a malformed contract, `Conflict` when a key is reused
/// for another body or project, and `Database` when the durable claim fails.
pub async fn begin(
    tx: &mut Transaction<'_, Postgres>,
    request: BeginRequest<'_>,
) -> AppResult<BeginOutcome> {
    validate_begin_request(request)?;
    assert_transaction_scope(tx, request.workspace_id, request.actor_id).await?;

    let inserted = sqlx::query_as::<_, IdempotencyRecord>(
        "insert into app.idempotency_records (
           workspace_id, project_id, actor_id, operation_key,
           idempotency_key, request_hash, status, locked_until, expires_at
         ) values (
           $1, $2, $3, $4, $5, $6, 'processing',
           now() + ($7::integer * interval '1 second'),
           now() + ($8::integer * interval '1 second')
         )
         on conflict (workspace_id, actor_id, operation_key, idempotency_key)
         do nothing
         returning id, public_id, workspace_id, project_id, actor_id,
                   operation_key, idempotency_key, request_hash, status,
                   response_status, response_body, error_code, lease_generation, locked_until,
                   expires_at, false as expired, false as reclaimable",
    )
    .bind(request.workspace_id)
    .bind(request.project_id)
    .bind(request.actor_id)
    .bind(request.operation_key)
    .bind(request.idempotency_key)
    .bind(request.request_hash)
    .bind(LEASE_DURATION_SECONDS)
    .bind(RETENTION_SECONDS)
    .fetch_optional(&mut **tx)
    .await?;

    if let Some(record) = inserted {
        return Ok(BeginOutcome::New {
            lease: record.into_lease()?,
            reclaimed: false,
            retrying_transient_failure: false,
        });
    }

    // `FOR UPDATE` serializes the conflict path with both the first insert and
    // any concurrent reclaimer. No mutation or provider call happens while
    // this row lock is held beyond the caller's short transaction.
    let record = sqlx::query_as::<_, IdempotencyRecord>(
        "select id, public_id, workspace_id, project_id, actor_id,
                operation_key, idempotency_key, request_hash, status,
                response_status, response_body, error_code, lease_generation, locked_until,
                expires_at, expires_at <= now() as expired,
                coalesce(locked_until <= now(), false) as reclaimable
         from app.idempotency_records
         where workspace_id = $1
           and actor_id = $2
           and operation_key = $3
           and idempotency_key = $4
         for update",
    )
    .bind(request.workspace_id)
    .bind(request.actor_id)
    .bind(request.operation_key)
    .bind(request.idempotency_key)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        AppError::Conflict("idempotency record changed while it was being claimed".into())
    })?;

    match classify_existing(&record, request) {
        ExistingDecision::ResetExpired => reset_expired(tx, record.id, request).await,
        ExistingDecision::ScopeConflict => Err(AppError::Conflict(
            "idempotency key was already used for a different project scope".into(),
        )),
        ExistingDecision::HashConflict => Err(AppError::Conflict(
            "idempotency key was already used with a different request body".into(),
        )),
        ExistingDecision::Reclaim => reclaim(tx, record).await,
        ExistingDecision::RetryFailed => retry_failed(tx, record).await,
        ExistingDecision::InProgress => Ok(BeginOutcome::InProgress {
            record_public_id: record.public_id,
            locked_until: record.locked_until.ok_or_else(|| {
                AppError::Internal("processing idempotency record has no lease".into())
            })?,
            expires_at: record.expires_at,
        }),
        ExistingDecision::Replay => Ok(BeginOutcome::Replay {
            record_public_id: record.public_id,
            response: record.stored_response()?,
            expires_at: record.expires_at,
        }),
    }
}

/// Keeps provider work owned when invoked as part of an idempotent command.
/// The service receives ownership errors and can mark its model run terminal.
///
/// # Errors
/// Returns provider errors or an ownership conflict.
pub fn with_optional_lease<'a, T: Send + 'a>(
    state: &'a crate::service::AppState,
    lease: Option<&'a IdempotencyLease>,
    command: impl Future<Output = AppResult<T>> + Send + 'a,
) -> impl Future<Output = AppResult<T>> + Send + 'a {
    let command = Box::pin(command);
    async move {
        if let Some(lease) = lease {
            with_lease(state, lease, command).await
        } else {
            command.await
        }
    }
}

/// Keeps a command owned while its future is active. Dropping this future also
/// drops the provider work and renewal loop; no detached task survives cancellation.
///
/// # Errors
/// Returns the command error, or a conflict if ownership cannot be renewed.
pub fn with_lease<'a, T: Send + 'a>(
    state: &'a crate::service::AppState,
    lease: &'a IdempotencyLease,
    command: impl Future<Output = AppResult<T>> + Send + 'a,
) -> impl Future<Output = AppResult<T>> + Send + 'a {
    let command = Box::pin(command);
    async move {
        let mut initial_tx = state.begin_request().await?;
        if !renew(&mut initial_tx, lease).await? {
            return Err(AppError::Conflict("command already finalized".into()));
        }
        initial_tx.commit().await?;
        let heartbeat = Box::pin(async {
            loop {
                tokio::time::sleep(Duration::from_secs(10)).await;
                let renewal = tokio::time::timeout(Duration::from_secs(10), async {
                    let mut tx = state.begin_request().await?;
                    let processing = renew(&mut tx, lease).await?;
                    tx.commit().await?;
                    Ok::<_, AppError>(processing)
                })
                .await
                .map_err(|_| AppError::Conflict("command lease renewal timed out".into()))??;
                if !renewal {
                    // The command finalized atomically before returning its value.
                    return std::future::pending::<AppResult<T>>().await;
                }
            }
        });
        tokio::select! {
            biased;
            result = command => result,
            result = heartbeat => result,
        }
    }
}

/// Extends only an unexpired lease owned by the same generation.
/// Returns false when this generation already finalized the command.
///
/// # Errors
/// Returns a conflict for an expired/reclaimed lease, or a database error.
pub async fn renew(
    tx: &mut Transaction<'_, Postgres>,
    lease: &IdempotencyLease,
) -> AppResult<bool> {
    assert_transaction_scope(tx, lease.workspace_id, lease.actor_id).await?;
    let renewed: Option<bool> = sqlx::query_scalar(
        "update app.idempotency_records
         set locked_until = case when status = 'processing'
               then now() + ($5::integer * interval '1 second') else locked_until end,
             expires_at = greatest(expires_at, now() + ($6::integer * interval '1 second')),
             updated_at = now()
         where public_id = $1 and workspace_id = $2 and actor_id = $3
           and lease_generation = $4
           and (status <> 'processing' or locked_until > now())
         returning status = 'processing'",
    )
    .bind(lease.record_public_id)
    .bind(lease.workspace_id)
    .bind(lease.actor_id)
    .bind(lease.generation)
    .bind(LEASE_DURATION_SECONDS)
    .bind(RETENTION_SECONDS)
    .fetch_optional(&mut **tx)
    .await?;
    renewed.ok_or_else(|| AppError::Conflict("command lease expired or was reclaimed".into()))
}

/// Persists the successful response produced by the lease owner.
///
/// Calling this function again with the same lease and response is safe. A
/// different final response or a reclaimed lease yields `409 Conflict`.
///
/// # Errors
///
/// Returns `Invalid` for an invalid HTTP status, `Conflict` for a stale lease
/// or mismatched final result, and `Database` when persistence fails.
pub async fn complete(
    tx: &mut Transaction<'_, Postgres>,
    lease: &IdempotencyLease,
    status_code: u16,
    body: Value,
) -> AppResult<StoredResponse> {
    validate_status_code(status_code)?;
    finalize(
        tx,
        lease,
        StoredResponse {
            status_code,
            body,
            failed: false,
            retryable: false,
            error_code: None,
        },
    )
    .await
}

/// Persists a sanitized failure response for deterministic replay or an
/// explicit same-command retry.
///
/// A permanent failure is always replayed. A retryable failure may only use a
/// 502, 503 or 504 status; its durable marker allows `begin` to atomically
/// transition the same record back to `processing`. Ambiguous provider
/// failures are never allowed to switch silently to another engine.
///
/// # Errors
///
/// Returns `Invalid` for an invalid status or error code, `Conflict` for a
/// stale lease or mismatched final result, and `Database` when persistence
/// fails.
pub async fn fail(
    tx: &mut Transaction<'_, Postgres>,
    lease: &IdempotencyLease,
    status_code: u16,
    error_code: &str,
    disposition: FailureDisposition,
    body: Value,
) -> AppResult<StoredResponse> {
    validate_status_code(status_code)?;
    validate_error_code(error_code)?;
    let retryable = match disposition {
        FailureDisposition::Permanent => false,
        FailureDisposition::Retryable if is_transient_gateway_status(status_code) => true,
        FailureDisposition::Retryable => {
            return Err(AppError::Invalid(
                "retryable idempotency failures require status 502, 503 or 504".into(),
            ));
        }
    };
    let body = mark_failure_retryability(body, retryable);
    finalize(
        tx,
        lease,
        StoredResponse {
            status_code,
            body,
            failed: true,
            retryable,
            error_code: Some(error_code.to_owned()),
        },
    )
    .await
}

async fn reset_expired(
    tx: &mut Transaction<'_, Postgres>,
    record_id: i64,
    request: BeginRequest<'_>,
) -> AppResult<BeginOutcome> {
    let record = sqlx::query_as::<_, IdempotencyRecord>(
        "update app.idempotency_records
         set lease_generation = gen_random_uuid(),
             project_id = $2,
             request_hash = $3,
             status = 'processing',
             response_status = null,
             response_body = null,
             error_code = null,
             locked_until = now() + ($4::integer * interval '1 second'),
             expires_at = now() + ($5::integer * interval '1 second'),
             created_at = now(),
             updated_at = now()
         where id = $1
         returning id, public_id, workspace_id, project_id, actor_id,
                   operation_key, idempotency_key, request_hash, status,
                   response_status, response_body, error_code, lease_generation, locked_until,
                   expires_at, false as expired, false as reclaimable",
    )
    .bind(record_id)
    .bind(request.project_id)
    .bind(request.request_hash)
    .bind(LEASE_DURATION_SECONDS)
    .bind(RETENTION_SECONDS)
    .fetch_one(&mut **tx)
    .await?;

    Ok(BeginOutcome::New {
        lease: record.into_lease()?,
        reclaimed: false,
        retrying_transient_failure: false,
    })
}

async fn reclaim(
    tx: &mut Transaction<'_, Postgres>,
    record: IdempotencyRecord,
) -> AppResult<BeginOutcome> {
    let record = sqlx::query_as::<_, IdempotencyRecord>(
        "update app.idempotency_records
         set lease_generation = gen_random_uuid(),
             locked_until = now() + ($2::integer * interval '1 second'),
             updated_at = now()
         where id = $1 and status = 'processing'
         returning id, public_id, workspace_id, project_id, actor_id,
                   operation_key, idempotency_key, request_hash, status,
                   response_status, response_body, error_code, lease_generation, locked_until,
                   expires_at, false as expired, false as reclaimable",
    )
    .bind(record.id)
    .bind(LEASE_DURATION_SECONDS)
    .fetch_one(&mut **tx)
    .await?;

    Ok(BeginOutcome::New {
        lease: record.into_lease()?,
        reclaimed: true,
        retrying_transient_failure: false,
    })
}

async fn retry_failed(
    tx: &mut Transaction<'_, Postgres>,
    record: IdempotencyRecord,
) -> AppResult<BeginOutcome> {
    // The row was selected FOR UPDATE by `begin`. Repeat every retryability
    // guard in the UPDATE so a future refactor cannot accidentally turn an
    // arbitrary failed command into a second execution.
    let record = sqlx::query_as::<_, IdempotencyRecord>(
        "update app.idempotency_records
         set lease_generation = gen_random_uuid(),
             status = 'processing',
             response_status = null,
             response_body = null,
             error_code = null,
             locked_until = now() + ($2::integer * interval '1 second'),
             updated_at = now()
         where id = $1
           and status = 'failed'
           and response_status in (502, 503, 504)
           and response_body ->> 'retryable' = 'true'
         returning id, public_id, workspace_id, project_id, actor_id,
                   operation_key, idempotency_key, request_hash, status,
                   response_status, response_body, error_code, lease_generation, locked_until,
                   expires_at, false as expired, false as reclaimable",
    )
    .bind(record.id)
    .bind(LEASE_DURATION_SECONDS)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        AppError::Conflict("retryable idempotency failure changed while it was claimed".into())
    })?;

    Ok(BeginOutcome::New {
        lease: record.into_lease()?,
        reclaimed: true,
        retrying_transient_failure: true,
    })
}

async fn finalize(
    tx: &mut Transaction<'_, Postgres>,
    lease: &IdempotencyLease,
    response: StoredResponse,
) -> AppResult<StoredResponse> {
    assert_transaction_scope(tx, lease.workspace_id, lease.actor_id).await?;
    let status = if response.failed {
        "failed"
    } else {
        "completed"
    };
    let status_code = i16::try_from(response.status_code)
        .map_err(|_| AppError::Invalid("response status is out of range".into()))?;
    let result = sqlx::query(
        "update app.idempotency_records
         set status = $8,
             response_status = $9,
             response_body = $10,
             error_code = $11,
             locked_until = null,
             updated_at = now()
         where public_id = $1
           and workspace_id = $2
           and project_id is not distinct from $3
           and actor_id = $4
           and operation_key = $5
           and idempotency_key = $6
           and request_hash = $7
           and status = 'processing'
           and lease_generation = $12",
    )
    .bind(lease.record_public_id)
    .bind(lease.workspace_id)
    .bind(lease.project_id)
    .bind(lease.actor_id)
    .bind(&lease.operation_key)
    .bind(&lease.idempotency_key)
    .bind(&lease.request_hash)
    .bind(status)
    .bind(status_code)
    .bind(&response.body)
    .bind(&response.error_code)
    .bind(lease.generation)
    .execute(&mut **tx)
    .await?;

    if result.rows_affected() == 1 {
        return Ok(response);
    }

    // Finalization itself is idempotent: if the database commit succeeded but
    // its acknowledgement was lost, the same response is accepted again.
    let existing = sqlx::query_as::<_, IdempotencyRecord>(
        "select id, public_id, workspace_id, project_id, actor_id,
                operation_key, idempotency_key, request_hash, status,
                response_status, response_body, error_code, lease_generation, locked_until,
                expires_at, expires_at <= now() as expired,
                coalesce(locked_until <= now(), false) as reclaimable
         from app.idempotency_records
         where public_id = $1
           and workspace_id = $2
           and project_id is not distinct from $3
           and actor_id = $4
           and operation_key = $5
           and idempotency_key = $6",
    )
    .bind(lease.record_public_id)
    .bind(lease.workspace_id)
    .bind(lease.project_id)
    .bind(lease.actor_id)
    .bind(&lease.operation_key)
    .bind(&lease.idempotency_key)
    .fetch_optional(&mut **tx)
    .await?;

    if let Some(existing) = existing
        && existing.request_hash == lease.request_hash
        && matches!(existing.status.as_str(), "completed" | "failed")
        && existing.stored_response()? == response
    {
        return Ok(response);
    }

    Err(AppError::Conflict(
        "idempotency lease is no longer active".into(),
    ))
}

async fn assert_transaction_scope(
    tx: &mut Transaction<'_, Postgres>,
    expected_workspace_id: i64,
    expected_actor_id: Uuid,
) -> AppResult<()> {
    let scope = sqlx::query_as::<_, TransactionScope>(
        "select app.current_actor_id() as actor_id,
                app.current_workspace_id() as workspace_id,
                app.current_workspace_role() as workspace_role",
    )
    .fetch_one(&mut **tx)
    .await?;
    validate_transaction_scope(&scope, expected_workspace_id, expected_actor_id)
}

fn validate_transaction_scope(
    scope: &TransactionScope,
    expected_workspace_id: i64,
    expected_actor_id: Uuid,
) -> AppResult<()> {
    if scope.actor_id != Some(expected_actor_id)
        || scope.workspace_id != Some(expected_workspace_id)
    {
        return Err(AppError::Internal(
            "idempotency transaction scope does not match its request context".into(),
        ));
    }
    if !matches!(scope.workspace_role.as_deref(), Some("owner" | "editor")) {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

fn validate_begin_request(request: BeginRequest<'_>) -> AppResult<()> {
    if request.workspace_id <= 0 {
        return Err(AppError::Invalid(
            "idempotency workspace identifier must be positive".into(),
        ));
    }
    if request.project_id.is_some_and(|project_id| project_id <= 0) {
        return Err(AppError::Invalid(
            "idempotency project identifier must be positive".into(),
        ));
    }
    validate_operation_key(request.operation_key)?;
    validate_idempotency_key(request.idempotency_key)?;
    validate_request_hash(request.request_hash)
}

fn validate_operation_key(value: &str) -> AppResult<()> {
    validate_trimmed_text(value, MAX_OPERATION_KEY_BYTES, "operation key")?;
    if !value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-' | b'/')
    }) {
        return Err(AppError::Invalid(
            "idempotency operation key contains unsupported characters".into(),
        ));
    }
    Ok(())
}

fn validate_idempotency_key(value: &str) -> AppResult<()> {
    validate_trimmed_text(value, MAX_IDEMPOTENCY_KEY_BYTES, "idempotency key")?;
    if !value.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(AppError::Invalid(
            "Idempotency-Key must contain visible ASCII characters only".into(),
        ));
    }
    Ok(())
}

fn validate_request_hash(value: &str) -> AppResult<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(AppError::Invalid(
            "idempotency request hash must be a lowercase SHA-256 digest".into(),
        ));
    }
    Ok(())
}

fn validate_error_code(value: &str) -> AppResult<()> {
    validate_trimmed_text(value, MAX_ERROR_CODE_BYTES, "idempotency error code")?;
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        return Err(AppError::Invalid(
            "idempotency error code contains unsupported characters".into(),
        ));
    }
    Ok(())
}

fn validate_trimmed_text(value: &str, maximum: usize, field: &str) -> AppResult<()> {
    if value.is_empty() || value.trim() != value {
        return Err(AppError::Invalid(format!(
            "idempotency {field} must not be blank or padded"
        )));
    }
    if value.len() > maximum {
        return Err(AppError::Invalid(format!(
            "idempotency {field} exceeds {maximum} bytes"
        )));
    }
    Ok(())
}

fn validate_status_code(status_code: u16) -> AppResult<()> {
    if !(100..=599).contains(&status_code) {
        return Err(AppError::Invalid(
            "idempotency response status must be between 100 and 599".into(),
        ));
    }
    Ok(())
}

fn is_transient_gateway_status(status_code: u16) -> bool {
    matches!(status_code, 502..=504)
}

fn mark_failure_retryability(mut body: Value, retryable: bool) -> Value {
    if let Value::Object(fields) = &mut body {
        fields.insert("retryable".into(), Value::Bool(retryable));
        body
    } else {
        serde_json::json!({ "message": body, "retryable": retryable })
    }
}

fn classify_existing(record: &IdempotencyRecord, request: BeginRequest<'_>) -> ExistingDecision {
    if record.expired {
        return ExistingDecision::ResetExpired;
    }
    if record.project_id != request.project_id {
        return ExistingDecision::ScopeConflict;
    }
    if record.request_hash != request.request_hash {
        return ExistingDecision::HashConflict;
    }
    match record.status.as_str() {
        "processing" if record.reclaimable => ExistingDecision::Reclaim,
        "processing" => ExistingDecision::InProgress,
        "failed" if record.is_retryable_failure() => ExistingDecision::RetryFailed,
        "completed" | "failed" => ExistingDecision::Replay,
        _ => ExistingDecision::HashConflict,
    }
}

fn canonicalize_json(value: &Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.iter().map(canonicalize_json).collect()),
        Value::Object(values) => {
            let sorted = values
                .iter()
                .map(|(key, value)| (key.clone(), canonicalize_json(value)))
                .collect::<BTreeMap<_, _>>();
            Value::Object(sorted.into_iter().collect())
        }
        scalar => scalar.clone(),
    }
}

impl IdempotencyRecord {
    fn into_lease(self) -> AppResult<IdempotencyLease> {
        let locked_until = self.locked_until.ok_or_else(|| {
            AppError::Internal("processing idempotency record has no lease".into())
        })?;
        Ok(IdempotencyLease {
            record_public_id: self.public_id,
            workspace_id: self.workspace_id,
            project_id: self.project_id,
            actor_id: self.actor_id,
            operation_key: self.operation_key,
            idempotency_key: self.idempotency_key,
            request_hash: self.request_hash,
            generation: self.lease_generation,
            locked_until,
            expires_at: self.expires_at,
        })
    }

    fn stored_response(&self) -> AppResult<StoredResponse> {
        let failed = match self.status.as_str() {
            "completed" => false,
            "failed" => true,
            _ => {
                return Err(AppError::Internal(
                    "unfinished idempotency record cannot be replayed".into(),
                ));
            }
        };
        let status = self.response_status.ok_or_else(|| {
            AppError::Internal("final idempotency record has no response status".into())
        })?;
        let status_code = u16::try_from(status)
            .map_err(|_| AppError::Internal("idempotency response status is invalid".into()))?;
        Ok(StoredResponse {
            status_code,
            body: self.response_body.clone().unwrap_or(Value::Null),
            failed,
            retryable: failed && self.is_retryable_failure(),
            error_code: self.error_code.clone(),
        })
    }

    fn is_retryable_failure(&self) -> bool {
        self.status == "failed"
            && self
                .response_status
                .and_then(|status| u16::try_from(status).ok())
                .is_some_and(is_transient_gateway_status)
            && self
                .response_body
                .as_ref()
                .and_then(|body| body.get("retryable"))
                .and_then(Value::as_bool)
                == Some(true)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;
    use serde_json::json;

    use super::*;

    const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    #[test]
    fn canonical_hash_ignores_object_key_order_recursively() {
        let first = json!({
            "task": {"project": "alpha", "graph": 4},
            "sources": [{"version": 2, "id": "a"}]
        });
        let second = json!({
            "sources": [{"id": "a", "version": 2}],
            "task": {"graph": 4, "project": "alpha"}
        });

        assert_eq!(hash_json(&first), hash_json(&second));
        assert_eq!(hash_json(&first).len(), 64);
    }

    #[test]
    fn canonical_hash_changes_with_request_data() {
        assert_ne!(
            hash_json(&json!({"context_pack_id": "one"})),
            hash_json(&json!({"context_pack_id": "two"}))
        );
    }

    #[test]
    fn validation_rejects_ambiguous_or_malformed_keys() {
        let actor_id = Uuid::new_v4();
        let valid = BeginRequest {
            workspace_id: 1,
            project_id: Some(2),
            actor_id,
            operation_key: "handoff.create",
            idempotency_key: "08b89e33-0771-4ccf-83dc-0591727e358b",
            request_hash: HASH_A,
        };
        assert!(validate_begin_request(valid).is_ok());
        assert!(
            validate_begin_request(BeginRequest {
                idempotency_key: " padded ",
                ..valid
            })
            .is_err()
        );
        assert!(
            validate_begin_request(BeginRequest {
                operation_key: "handoff create",
                ..valid
            })
            .is_err()
        );
        assert!(
            validate_begin_request(BeginRequest {
                request_hash: "ABC",
                ..valid
            })
            .is_err()
        );
        assert!(
            validate_begin_request(BeginRequest {
                idempotency_key: &"x".repeat(MAX_IDEMPOTENCY_KEY_BYTES + 1),
                ..valid
            })
            .is_err()
        );
    }

    #[test]
    fn transaction_scope_requires_matching_actor_workspace_and_mutating_role() {
        let actor_id = Uuid::new_v4();
        for role in ["owner", "editor"] {
            let scope = TransactionScope {
                actor_id: Some(actor_id),
                workspace_id: Some(42),
                workspace_role: Some(role.into()),
            };
            assert!(validate_transaction_scope(&scope, 42, actor_id).is_ok());
        }

        let viewer = TransactionScope {
            actor_id: Some(actor_id),
            workspace_id: Some(42),
            workspace_role: Some("viewer".into()),
        };
        assert!(matches!(
            validate_transaction_scope(&viewer, 42, actor_id),
            Err(AppError::Forbidden)
        ));

        let wrong_workspace = TransactionScope {
            actor_id: Some(actor_id),
            workspace_id: Some(7),
            workspace_role: Some("owner".into()),
        };
        assert!(matches!(
            validate_transaction_scope(&wrong_workspace, 42, actor_id),
            Err(AppError::Internal(_))
        ));
    }

    #[test]
    fn existing_processing_record_is_in_progress_or_reclaimed() {
        let active = record("processing", HASH_A, Some(2), false, false);
        let request = request(HASH_A, Some(2));
        assert_eq!(
            classify_existing(&active, request),
            ExistingDecision::InProgress
        );

        let abandoned = record("processing", HASH_A, Some(2), false, true);
        assert_eq!(
            classify_existing(&abandoned, request),
            ExistingDecision::Reclaim
        );
    }

    #[test]
    fn collision_checks_body_and_project_scope() {
        let existing = record("completed", HASH_A, Some(2), false, false);
        assert_eq!(
            classify_existing(&existing, request(HASH_B, Some(2))),
            ExistingDecision::HashConflict
        );
        assert_eq!(
            classify_existing(&existing, request(HASH_A, Some(3))),
            ExistingDecision::ScopeConflict
        );
    }

    #[test]
    fn final_response_replays_but_expired_record_is_fresh() {
        let completed = record("completed", HASH_A, Some(2), false, false);
        assert_eq!(
            classify_existing(&completed, request(HASH_A, Some(2))),
            ExistingDecision::Replay
        );

        let expired = record("completed", HASH_A, Some(2), true, false);
        assert_eq!(
            classify_existing(&expired, request(HASH_B, Some(9))),
            ExistingDecision::ResetExpired
        );
    }

    #[test]
    fn only_explicit_transient_gateway_failures_resume_the_same_command() {
        for status in [502, 503, 504] {
            let mut retryable = record("failed", HASH_A, Some(2), false, false);
            retryable.response_status = Some(status);
            retryable.response_body = Some(json!({
                "code": "provider_unavailable",
                "retryable": true,
            }));
            retryable.error_code = Some("provider_unavailable".into());
            assert_eq!(
                classify_existing(&retryable, request(HASH_A, Some(2))),
                ExistingDecision::RetryFailed
            );
            assert!(
                retryable
                    .stored_response()
                    .expect("stored failure")
                    .retryable
            );
        }

        let mut unmarked_gateway = record("failed", HASH_A, Some(2), false, false);
        unmarked_gateway.response_status = Some(503);
        unmarked_gateway.response_body = Some(json!({"code": "provider_unavailable"}));
        unmarked_gateway.error_code = Some("provider_unavailable".into());
        assert_eq!(
            classify_existing(&unmarked_gateway, request(HASH_A, Some(2))),
            ExistingDecision::Replay
        );

        let mut marked_permanent = record("failed", HASH_A, Some(2), false, false);
        marked_permanent.response_status = Some(422);
        marked_permanent.response_body = Some(json!({
            "code": "invalid_request",
            "retryable": true,
        }));
        marked_permanent.error_code = Some("invalid_request".into());
        assert_eq!(
            classify_existing(&marked_permanent, request(HASH_A, Some(2))),
            ExistingDecision::Replay
        );
    }

    #[test]
    fn server_overwrites_retryability_in_the_failure_body() {
        assert_eq!(
            mark_failure_retryability(
                json!({"code": "provider_unavailable", "retryable": false}),
                true,
            ),
            json!({"code": "provider_unavailable", "retryable": true})
        );
        assert_eq!(
            mark_failure_retryability(json!("bad request"), false),
            json!({"message": "bad request", "retryable": false})
        );
    }

    #[test]
    fn completed_and_failed_records_have_replayable_json() {
        let mut completed = record("completed", HASH_A, Some(2), false, false);
        completed.response_status = Some(201);
        completed.response_body = Some(json!({"handoff_id": "h-1"}));
        assert_eq!(
            completed.stored_response().expect("completed response"),
            StoredResponse {
                status_code: 201,
                body: json!({"handoff_id": "h-1"}),
                failed: false,
                retryable: false,
                error_code: None,
            }
        );

        let mut failed = record("failed", HASH_A, Some(2), false, false);
        failed.response_status = Some(503);
        failed.response_body = Some(json!({"code": "provider_unavailable", "retryable": false}));
        failed.error_code = Some("provider_unavailable".into());
        let response = failed.stored_response().expect("failed response");
        assert!(response.failed);
        assert!(!response.retryable);
    }

    fn request(request_hash: &str, project_id: Option<i64>) -> BeginRequest<'_> {
        BeginRequest {
            workspace_id: 1,
            project_id,
            actor_id: Uuid::nil(),
            operation_key: "handoff.create",
            idempotency_key: "test-key",
            request_hash,
        }
    }

    fn record(
        status: &str,
        request_hash: &str,
        project_id: Option<i64>,
        expired: bool,
        reclaimable: bool,
    ) -> IdempotencyRecord {
        let now = Utc::now();
        IdempotencyRecord {
            id: 1,
            public_id: Uuid::new_v4(),
            workspace_id: 1,
            project_id,
            actor_id: Uuid::nil(),
            operation_key: "handoff.create".into(),
            idempotency_key: "test-key".into(),
            request_hash: request_hash.into(),
            status: status.into(),
            response_status: None,
            response_body: None,
            error_code: None,
            lease_generation: Uuid::new_v4(),
            locked_until: (status == "processing").then_some(now + TimeDelta::seconds(30)),
            expires_at: now + TimeDelta::hours(24),
            expired,
            reclaimable,
        }
    }
}
