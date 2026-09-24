//! A captured conversation is untrusted working memory, never confirmed knowledge.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

const MAX_MESSAGES: usize = 24;
const MAX_MESSAGE_BYTES: usize = 8 * 1024;
const MAX_SERIALIZED_BYTES: usize = 32 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationMessage {
    pub message_public_id: Uuid,
    pub author_actor_id: Option<Uuid>,
    pub role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub content_truncated: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationContext {
    pub session_public_id: Uuid,
    pub current_message_public_id: Option<Uuid>,
    pub trust: String,
    pub messages: Vec<ConversationMessage>,
    pub available_messages: i64,
    pub omitted_messages: i64,
    pub truncated_messages: usize,
    pub max_messages: usize,
    pub max_message_bytes: usize,
    pub max_serialized_bytes: usize,
}

impl ConversationContext {
    #[must_use]
    pub fn empty(session_public_id: Uuid, current_message_public_id: Uuid) -> Self {
        Self {
            session_public_id,
            current_message_public_id: Some(current_message_public_id),
            trust: "unconfirmed_conversation".into(),
            messages: vec![],
            available_messages: 0,
            omitted_messages: 0,
            truncated_messages: 0,
            max_messages: MAX_MESSAGES,
            max_message_bytes: MAX_MESSAGE_BYTES,
            max_serialized_bytes: MAX_SERIALIZED_BYTES,
        }
    }

    pub(crate) fn provenance(&self) -> Value {
        json!({
            "session_public_id": self.session_public_id,
            "current_message_public_id": self.current_message_public_id,
            "trust": self.trust,
            "message_public_ids": self.messages.iter().map(|m| m.message_public_id).collect::<Vec<_>>(),
            "available_messages": self.available_messages,
            "included_messages": self.messages.len(),
            "omitted_messages": self.omitted_messages,
            "truncated_messages": self.truncated_messages,
            "max_messages": self.max_messages,
            "max_message_bytes": self.max_message_bytes,
            "max_serialized_bytes": self.max_serialized_bytes,
        })
    }

    fn refresh_counts(&mut self) {
        self.omitted_messages = self.available_messages
            - i64::try_from(self.messages.len()).expect("at most 24 messages");
        self.truncated_messages = self.messages.iter().filter(|m| m.content_truncated).count();
    }

    fn fits(&self) -> AppResult<bool> {
        let bytes =
            serde_json::to_vec(self).map_err(|error| AppError::Internal(error.to_string()))?;
        Ok(bytes.len() <= MAX_SERIALIZED_BYTES)
    }
}

#[derive(sqlx::FromRow)]
struct MessageRow {
    message_public_id: Uuid,
    author_actor_id: Option<Uuid>,
    role: String,
    content: String,
    original_bytes: i32,
    created_at: DateTime<Utc>,
    available_messages: i64,
}

/// Prepare the immutable first snapshot before inserting the user message.
/// Persist this value with that message; retries restore it instead of reading
/// any transcript changes committed since the original turn.
pub(crate) async fn prepare(
    tx: &mut Transaction<'_, Postgres>,
    session_id: i64,
    session_public_id: Uuid,
    current_message_public_id: Uuid,
) -> AppResult<ConversationContext> {
    let rows = read_rows(tx, session_id, i64::MAX).await?;
    bound(session_public_id, Some(current_message_public_id), rows)
}

pub(crate) fn restore(
    snapshot: Value,
    session_public_id: Uuid,
    current_message_public_id: Uuid,
) -> AppResult<ConversationContext> {
    let invalid = || {
        AppError::Conflict(
        "L’historique capturé pour cet ancien message n’est pas disponible. Envoyez un nouveau message pour poursuivre avec le contexte actuel.".into(),
    )
    };
    let captured: ConversationContext = serde_json::from_value(snapshot).map_err(|_| invalid())?;
    if captured.session_public_id != session_public_id
        || captured.current_message_public_id != Some(current_message_public_id)
        || captured.trust != "unconfirmed_conversation"
        || captured.max_messages != MAX_MESSAGES
        || captured.max_message_bytes != MAX_MESSAGE_BYTES
        || captured.max_serialized_bytes != MAX_SERIALIZED_BYTES
        || captured.messages.len() > MAX_MESSAGES
        || captured.available_messages < 0
        || captured.omitted_messages < 0
        || captured
            .available_messages
            .checked_sub(i64::try_from(captured.messages.len()).expect("bounded messages"))
            != Some(captured.omitted_messages)
        || captured.truncated_messages
            != captured
                .messages
                .iter()
                .filter(|m| m.content_truncated)
                .count()
        || captured.messages.iter().any(|m| {
            m.content.len() > MAX_MESSAGE_BYTES
                || !matches!(m.role.as_str(), "user" | "assistant")
                || m.message_public_id == current_message_public_id
        })
        || !captured.fits()?
    {
        return Err(invalid());
    }
    Ok(captured)
}

/// Capture the latest visible transcript for an explicit document-generation
/// command that has no new user message. Uses one statement snapshot.
pub(crate) async fn capture_latest(
    tx: &mut Transaction<'_, Postgres>,
    session_id: i64,
    session_public_id: Uuid,
) -> AppResult<ConversationContext> {
    let rows = read_rows(tx, session_id, i64::MAX).await?;
    bound(session_public_id, None, rows)
}

/// Append-only messages let the newest identity AND count detect changes,
/// including a transaction that commits an older allocated identity later.
pub(crate) async fn verify_latest(
    tx: &mut Transaction<'_, Postgres>,
    session_id: i64,
    captured: &ConversationContext,
) -> AppResult<bool> {
    if captured.current_message_public_id.is_some() {
        return Ok(false);
    }
    let current: Option<(Uuid, i64)> = sqlx::query_as(
        "select public_id,(select count(*) from app.messages counter
             where counter.workspace_id=app.current_workspace_id() and counter.session_id=$1
               and counter.role in ('user','assistant')) from app.messages
         where workspace_id=app.current_workspace_id() and session_id=$1
           and role in ('user','assistant') order by id desc limit 1",
    )
    .bind(session_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(current.map_or((None, 0), |(id, count)| (Some(id), count))
        == (
            captured.messages.last().map(|m| m.message_public_id),
            captured.available_messages,
        ))
}

async fn read_rows(
    tx: &mut Transaction<'_, Postgres>,
    session_id: i64,
    before_id: i64,
) -> AppResult<Vec<MessageRow>> {
    // Bound rows and text in PostgreSQL as well as the serialized model payload.
    // Count and rows use the same statement snapshot. The scalar count reads
    // no text and avoids buffering an entire transcript in a window aggregate.
    // No provider-time lazy reads can import a later message or another session.
    Ok(sqlx::query_as::<_, MessageRow>(
        "select public_id as message_public_id,author_actor_id,role,
                left(content,8192) as content,octet_length(content) as original_bytes,
                created_at,(select count(*) from app.messages counter
                  where counter.workspace_id=app.current_workspace_id() and counter.session_id=$1
                    and counter.id<$2 and counter.role in ('user','assistant')) as available_messages
         from app.messages
         where workspace_id=app.current_workspace_id() and session_id=$1
           and id<$2 and role in ('user','assistant')
         order by id desc limit 24",
    )
    .bind(session_id)
    .bind(before_id)
    .fetch_all(&mut **tx)
    .await?)
}

fn prefix(text: &str, bytes: usize) -> String {
    let mut end = bytes.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].into()
}

fn bound(
    session_public_id: Uuid,
    current_public_id: Option<Uuid>,
    rows_newest_first: Vec<MessageRow>,
) -> AppResult<ConversationContext> {
    let mut window = ConversationContext::empty(session_public_id, Uuid::nil());
    window.current_message_public_id = current_public_id;
    window.available_messages = rows_newest_first
        .first()
        .map_or(0, |r| r.available_messages);
    window.refresh_counts();
    for row in rows_newest_first.into_iter().take(MAX_MESSAGES) {
        let content = prefix(&row.content, MAX_MESSAGE_BYTES);
        let content_truncated =
            i64::try_from(content.len()).expect("bounded text") < i64::from(row.original_bytes);
        window.messages.insert(
            0,
            ConversationMessage {
                message_public_id: row.message_public_id,
                author_actor_id: row.author_actor_id,
                role: row.role,
                content,
                created_at: row.created_at,
                content_truncated,
            },
        );
        window.refresh_counts();
        if window.fits()? {
            continue;
        }
        // Fit the oldest included excerpt, never discard newer messages. JSON
        // escaped bytes count against the limit, not just the raw UTF-8 text.
        let content = std::mem::take(&mut window.messages[0].content);
        window.messages[0].content_truncated = true;
        window.refresh_counts();
        if !window.fits()? {
            window.messages.remove(0);
            window.refresh_counts();
            break;
        }
        let mut low = 0;
        let mut high = content.len();
        while low < high {
            let middle = low + (high - low).div_ceil(2);
            window.messages[0].content = prefix(&content, middle);
            if window.fits()? {
                low = middle;
            } else {
                high = middle - 1;
            }
        }
        window.messages[0].content = prefix(&content, low);
        if window.messages[0].content.is_empty() {
            window.messages.remove(0);
        }
        window.refresh_counts();
        break;
    }
    Ok(window)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(count: usize, text: &str) -> Vec<MessageRow> {
        (0..count)
            .rev()
            .map(|n| MessageRow {
                message_public_id: Uuid::from_u128(n as u128 + 1),
                author_actor_id: None,
                role: if n % 2 == 0 { "user" } else { "assistant" }.into(),
                content: text.into(),
                original_bytes: i32::try_from(text.len()).unwrap(),
                created_at: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
                available_messages: i64::try_from(count).unwrap(),
            })
            .collect()
    }

    #[test]
    fn empty_and_recent_windows_preserve_identity_order_and_truth_boundary() {
        let session = Uuid::from_u128(100);
        let current = Uuid::from_u128(101);
        let empty = bound(session, Some(current), vec![]).unwrap();
        assert!(empty.messages.is_empty());
        assert_eq!(empty.omitted_messages, 0);
        let window = bound(
            session,
            Some(current),
            rows(30, "Hypothèse [FICTIF] non validée"),
        )
        .unwrap();
        assert_eq!(window.session_public_id, session);
        assert_eq!(window.current_message_public_id, Some(current));
        assert_eq!(window.trust, "unconfirmed_conversation");
        assert_eq!(window.messages.len(), 24);
        assert_eq!(window.messages[0].message_public_id, Uuid::from_u128(7));
        assert_eq!(window.messages[23].message_public_id, Uuid::from_u128(30));
        assert_eq!(window.omitted_messages, 6);
        assert_eq!(window.truncated_messages, 0);
        assert!(
            window
                .messages
                .windows(2)
                .all(|w| w[0].message_public_id < w[1].message_public_id)
        );
        assert!(!window.provenance().to_string().contains("Hypothèse"));
    }

    #[test]
    fn utf8_and_json_escaping_fit_the_exact_budget_deterministically() {
        let text = "é💡\n\"\\\u{0001}".repeat(4_000);
        let window = bound(Uuid::nil(), None, rows(30, &text)).unwrap();
        assert!(serde_json::to_vec(&window).unwrap().len() <= MAX_SERIALIZED_BYTES);
        assert!(window.truncated_messages > 0);
        assert!(window.omitted_messages > 0);
        assert_eq!(
            window.messages.last().unwrap().message_public_id,
            Uuid::from_u128(30)
        );
        for message in &window.messages {
            assert!(message.content.len() <= MAX_MESSAGE_BYTES);
            assert!(text.starts_with(&message.content));
            assert!(!message.content.is_empty());
        }
        let again = bound(Uuid::nil(), None, rows(30, &text)).unwrap();
        assert_eq!(
            serde_json::to_vec(&window).unwrap(),
            serde_json::to_vec(&again).unwrap()
        );
        assert_eq!(
            window.available_messages,
            window.omitted_messages + i64::try_from(window.messages.len()).unwrap()
        );
    }

    #[test]
    fn durable_snapshot_restore_rejects_legacy_wrong_scope_and_invalid_bounds() {
        let session = Uuid::from_u128(100);
        let current = Uuid::from_u128(101);
        let captured = bound(session, Some(current), rows(2, "[FICTIF] hypothesis")).unwrap();
        let stored = serde_json::to_value(&captured).unwrap();
        let restored = restore(stored.clone(), session, current).unwrap();
        assert_eq!(serde_json::to_value(restored).unwrap(), stored);
        assert!(restore(Value::Null, session, current).is_err());
        assert!(restore(stored.clone(), Uuid::nil(), current).is_err());
        assert!(restore(stored.clone(), session, Uuid::nil()).is_err());
        let mut invalid = stored.clone();
        invalid["omitted_messages"] = json!(100);
        assert!(restore(invalid, session, current).is_err());
        let mut invalid = stored.clone();
        invalid["messages"][0]["role"] = json!("system");
        assert!(restore(invalid, session, current).is_err());
        let mut invalid = stored;
        invalid["messages"][0]["content"] = json!("x".repeat(MAX_MESSAGE_BYTES + 1));
        assert!(restore(invalid, session, current).is_err());
    }

    #[test]
    fn sql_excerpt_truncation_is_not_mistaken_for_a_complete_message() {
        let mut input = rows(1, "extrait");
        input[0].original_bytes = 80_000;
        let window = bound(Uuid::nil(), None, input).unwrap();
        assert_eq!(window.truncated_messages, 1);
        assert_eq!(window.omitted_messages, 0);
    }
}
