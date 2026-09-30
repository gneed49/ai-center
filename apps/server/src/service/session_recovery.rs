//! Read-only receipts for the current author's messages. No provider is invoked.
use super::{AppResult, Postgres, SessionRecord, Transaction, idempotency};
use crate::models::{MessageCommandView, SendMessage};
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct CommandRow {
    message_public_id: Uuid,
    client_message_id: Uuid,
    idempotency_key: String,
    submitted_content: String,
    request_hash: String,
    command_status: String,
    response_status: Option<i16>,
    response_body: Option<Value>,
    error_code: Option<String>,
    locked_until: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    observed_at: DateTime<Utc>,
    answered: bool,
    may_write: bool,
}

pub(super) async fn commands(
    tx: &mut Transaction<'_, Postgres>,
    session: &SessionRecord,
) -> AppResult<Vec<MessageCommandView>> {
    // These explicit actor predicates remain required even though session
    // messages are shared. Never return another member's command body or key.
    let rows: Vec<CommandRow> = sqlx::query_as(
        "select m.public_id as message_public_id,m.client_message_id,m.submitted_content,
         r.idempotency_key,r.request_hash,r.status as command_status,r.response_status,
         r.response_body,r.error_code,r.locked_until,r.updated_at,r.expires_at,now() as observed_at,
         exists(select 1 from app.messages answer where answer.session_id=m.session_id
           and answer.role='assistant' and answer.client_message_id=m.client_message_id) as answered,
         (p.status='active' and app.has_workspace_role(p.workspace_id,array['owner','editor'])) as may_write
         from app.messages m join app.idempotency_records r on r.public_id=m.command_public_id
           and r.workspace_id=m.workspace_id and r.project_id=m.project_id and r.actor_id=m.author_actor_id
         join app.projects p on p.id=m.project_id
         where m.session_id=$1 and m.workspace_id=app.current_workspace_id()
           and m.role='user' and m.author_actor_id=app.current_actor_id()
           and r.actor_id=app.current_actor_id() and r.operation_key='session.send_message'
           and m.client_message_id is not null and m.submitted_content is not null
         order by m.id desc limit 50",
    )
    .bind(session.id)
    .fetch_all(&mut **tx)
    .await?;
    rows.into_iter()
        .filter_map(|row| receipt(session.public_id, row).transpose())
        .collect()
}

fn receipt(session_id: Uuid, row: CommandRow) -> AppResult<Option<MessageCommandView>> {
    let Ok(idempotency_key) = Uuid::parse_str(&row.idempotency_key) else {
        return Ok(None);
    };
    let input = SendMessage {
        content: row.submitted_content.clone(),
        client_message_id: row.client_message_id,
    };
    // A retained command may have been reset after its retention deadline.
    // Its new operation body must never become a receipt for the old message.
    if row.request_hash != idempotency::hash_request(&(session_id, &input))? {
        return Ok(None);
    }
    let retryable = row
        .response_body
        .as_ref()
        .and_then(|body| body.get("retryable"))
        .and_then(Value::as_bool)
        == Some(true)
        && matches!(row.response_status, Some(429 | 502..=504));
    let status = if row.answered {
        "completed"
    } else if row.expires_at <= row.observed_at {
        "expired"
    } else {
        match row.command_status.as_str() {
            "processing"
                if row
                    .locked_until
                    .is_some_and(|until| until > row.observed_at) =>
            {
                "processing"
            }
            "processing" => "interrupted",
            "failed" if retryable => "retryable",
            "completed" => "completed",
            _ => "failed",
        }
    };
    let delay = row
        .response_body
        .as_ref()
        .and_then(|body| body.get("retry_after_seconds"))
        .and_then(Value::as_u64);
    // Delay is relative to the persisted failure, not every subsequent GET.
    let retry_after_seconds = delay.map(|seconds| {
        seconds.saturating_sub(
            u64::try_from(
                row.observed_at
                    .signed_duration_since(row.updated_at)
                    .num_seconds(),
            )
            .unwrap_or_default(),
        )
    });
    let can_retry = row.may_write
        && matches!(status, "interrupted" | "retryable")
        && retry_after_seconds.unwrap_or(0) == 0;
    let error_message = row
        .response_body
        .as_ref()
        .and_then(|body| body.get("message"))
        .and_then(Value::as_str)
        .map(|message| message.chars().take(1024).collect());
    Ok(Some(MessageCommandView {
        message_public_id: row.message_public_id,
        client_message_id: row.client_message_id,
        idempotency_key,
        submitted_content: row.submitted_content,
        status: status.into(),
        can_retry,
        locked_until: row.locked_until,
        retry_after_seconds,
        error_code: row.error_code,
        error_message,
        updated_at: row.updated_at,
    }))
}
