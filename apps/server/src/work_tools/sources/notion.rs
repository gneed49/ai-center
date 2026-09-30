use super::super::models::{
    ExistingReadErrorCode as Code, ExistingReadFailure as Failure, ExistingToolSnapshot,
    OmissionReason as O, SourceMetadata, ToolProvider,
};
use super::{
    ExistingToolReader,
    linear::{bounded_text, coverage, finish, invalid, optional_date, text, uuid_field},
};
use secrecy::SecretString;
use serde_json::Value;
use uuid::Uuid;
impl ExistingToolReader {
    pub(super) async fn read_notion(
        &self,
        secret: &SecretString,
        id: Uuid,
    ) -> Result<ExistingToolSnapshot, Failure> {
        let path = format!("/v1/pages/{id}");
        let before = self
            .request(ToolProvider::Notion, secret, &path, None)
            .await?;
        let before = metadata(&before, id)?;
        let markdown = self
            .request(
                ToolProvider::Notion,
                secret,
                &format!("{path}/markdown?include_transcript=false"),
                None,
            )
            .await?;
        if text(&markdown, "object")? != "page_markdown" || uuid_field(&markdown, "id")? != id {
            return Err(Failure::new(Code::RemoteIdentityMismatch));
        }
        let body = text(&markdown, "markdown")?;
        let truncated = markdown
            .get("truncated")
            .and_then(Value::as_bool)
            .ok_or_else(invalid)?;
        let unknown = markdown
            .get("unknown_block_ids")
            .and_then(Value::as_array)
            .ok_or_else(invalid)?;
        if unknown.len() > 100
            || unknown.iter().any(|id| {
                id.as_str()
                    .and_then(|id| Uuid::parse_str(id).ok())
                    .is_none()
            })
        {
            return Err(invalid());
        }
        let after = self
            .request(ToolProvider::Notion, secret, &path, None)
            .await?;
        let after = metadata(&after, id)?;
        if before != after {
            return Err(Failure::new(Code::RemoteChangedDuringRead));
        }
        let mut omissions = vec![
            O::CommentsNotRead,
            O::AttachmentsNotRead,
            O::RelatedObjectsNotRead,
            O::PropertiesNotRead,
            O::EmbeddedContentNotRead,
            O::TranscriptsNotRead,
        ];
        if before.updated_at.is_none() {
            omissions.push(O::RemoteDateUnavailable);
        }
        if truncated {
            omissions.push(O::ProviderTruncated);
        }
        if !unknown.is_empty() || body.contains("<unknown") {
            omissions.push(O::UnknownBlocks);
        }
        let (title, body) = bounded_text(&before.title, body, &mut omissions)?;
        finish(ExistingToolSnapshot {
            external_id: id,
            canonical_url: format!("https://www.notion.so/{}", id.simple()),
            title,
            body_markdown: body,
            remote_updated_at: before.updated_at,
            coverage: coverage(&omissions),
            omission_reasons: omissions,
            metadata: before.parent,
        })
    }
}
#[derive(PartialEq, Eq)]
struct Metadata {
    title: String,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    parent: SourceMetadata,
}
fn metadata(value: &Value, expected: Uuid) -> Result<Metadata, Failure> {
    if text(value, "object")? != "page" || uuid_field(value, "id")? != expected {
        return Err(Failure::new(Code::RemoteIdentityMismatch));
    }
    let mut known_availability = false;
    for key in ["archived", "in_trash"] {
        if let Some(value) = value.get(key) {
            known_availability = true;
            if value.as_bool().ok_or_else(invalid)? {
                return Err(Failure::new(Code::RemoteNotFound));
            }
        }
    }
    if !known_availability {
        return Err(invalid());
    }
    let parent = value.get("parent").ok_or_else(invalid)?;
    let kind = text(parent, "type")?;
    let parent_id = match kind {
        "workspace" => {
            if parent.get("workspace").and_then(Value::as_bool) != Some(true) {
                return Err(invalid());
            }
            None
        }
        "page_id" | "data_source_id" => Some(uuid_field(parent, kind)?),
        _ => return Err(invalid()),
    };
    let properties = value
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(invalid)?;
    let mut titles = properties
        .values()
        .filter(|property| property.get("type").and_then(Value::as_str) == Some("title"));
    let title = titles
        .next()
        .ok_or_else(invalid)?
        .get("title")
        .and_then(Value::as_array)
        .ok_or_else(invalid)?;
    if titles.next().is_some() {
        return Err(invalid());
    }
    let mut title_text = String::new();
    for piece in title {
        title_text.push_str(text(piece, "plain_text")?);
    }
    // An untitled page is valid: preserve the empty observed title. The UI may
    // display its own fallback label without persisting an invented source title.
    Ok(Metadata {
        title: title_text,
        updated_at: optional_date(value, "last_edited_time")?,
        parent: SourceMetadata::Notion {
            parent_type: kind.into(),
            parent_id,
        },
    })
}
