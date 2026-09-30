use super::super::{
    locator::{LinearIssueKey, linear_identifier, linear_url},
    models::{
        ExistingReadErrorCode as Code, ExistingReadFailure as Failure, ExistingToolSnapshot,
        LinearState, MAX_SNAPSHOT_BYTES, MAX_TEXT_BYTES, OmissionReason as O, SourceCoverage,
        SourceMetadata, ToolProvider,
    },
};
use super::ExistingToolReader;
use chrono::{DateTime, Utc};
use secrecy::SecretString;
use serde_json::{Value, json};
use uuid::Uuid;
const ISSUE_QUERY: &str = "query ExistingIssue($id: String!) { issue(id: $id) { id identifier url title description updatedAt team { id } state { id name type } } }";
impl ExistingToolReader {
    pub(super) async fn read_linear(
        &self,
        secret: &SecretString,
        key: &LinearIssueKey,
        workspace: Option<&str>,
    ) -> Result<ExistingToolSnapshot, Failure> {
        let key_value = match key {
            LinearIssueKey::Id(id) => id.to_string(),
            LinearIssueKey::Identifier(key) => key.clone(),
        };
        let value = self
            .request(
                ToolProvider::Linear,
                secret,
                "/graphql",
                Some(&json!({"query":ISSUE_QUERY,"variables":{"id":key_value}})),
            )
            .await?;
        let issue = value.pointer("/data/issue").ok_or_else(invalid)?;
        if issue.is_null() {
            return Err(Failure::new(Code::RemoteNotFound));
        }
        let id = uuid_field(issue, "id")?;
        let identifier = linear_identifier(text(issue, "identifier")?).ok_or_else(invalid)?;
        let (actual_workspace, url_identifier, url) = linear_url(text(issue, "url")?)
            .map_err(|_| Failure::new(Code::RemoteIdentityMismatch))?;
        if url_identifier != identifier
            || workspace.is_some_and(|expected| expected != actual_workspace)
            || match key {
                LinearIssueKey::Id(expected) => *expected != id,
                LinearIssueKey::Identifier(expected) => *expected != identifier,
            }
        {
            return Err(Failure::new(Code::RemoteIdentityMismatch));
        }
        let team = issue.get("team").ok_or_else(invalid)?;
        let state = match issue.get("state") {
            Some(Value::Null) => None,
            Some(value) => Some(LinearState {
                id: uuid_field(value, "id")?,
                name: bounded_metadata(text(value, "name")?)?,
                r#type: bounded_metadata(text(value, "type")?)?,
            }),
            None => return Err(invalid()),
        };
        let mut omissions = vec![
            O::CommentsNotRead,
            O::AttachmentsNotRead,
            O::RelatedObjectsNotRead,
        ];
        let date = optional_date(issue, "updatedAt")?;
        if date.is_none() {
            omissions.push(O::RemoteDateUnavailable);
        }
        let body = match issue.get("description") {
            Some(Value::Null) => "",
            Some(Value::String(body)) => body.as_str(),
            _ => return Err(invalid()),
        };
        if text(issue, "title")?.trim().is_empty() {
            return Err(invalid());
        }
        let (title, body) = bounded_text(text(issue, "title")?, body, &mut omissions)?;
        finish(ExistingToolSnapshot {
            external_id: id,
            canonical_url: url,
            title,
            body_markdown: body,
            remote_updated_at: date,
            coverage: coverage(&omissions),
            omission_reasons: omissions,
            metadata: SourceMetadata::Linear {
                identifier,
                team_id: uuid_field(team, "id")?,
                state,
            },
        })
    }
}
pub(super) fn invalid() -> Failure {
    Failure::new(Code::RemoteResponseInvalid)
}
pub(super) fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, Failure> {
    value.get(key).and_then(Value::as_str).ok_or_else(invalid)
}
pub(super) fn uuid_field(value: &Value, key: &str) -> Result<Uuid, Failure> {
    Uuid::parse_str(text(value, key)?)
        .ok()
        .filter(|id| !id.is_nil())
        .ok_or_else(invalid)
}
pub(super) fn optional_date(value: &Value, key: &str) -> Result<Option<DateTime<Utc>>, Failure> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(date)) => DateTime::parse_from_rfc3339(date)
            .map(|date| Some(date.with_timezone(&Utc)))
            .map_err(|_| invalid()),
        _ => Err(invalid()),
    }
}
fn bounded_metadata(value: &str) -> Result<String, Failure> {
    if value.len() > 1024 || value.contains('\0') {
        Err(invalid())
    } else {
        Ok(value.into())
    }
}
fn truncate_bytes(value: &str, limit: usize) -> &str {
    let mut end = value.len().min(limit);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}
pub(super) fn bounded_text(
    title: &str,
    body: &str,
    omissions: &mut Vec<O>,
) -> Result<(String, String), Failure> {
    if title.contains('\0') || body.contains('\0') {
        return Err(invalid());
    }
    let normalized_title = title.replace("\r\n", "\n").replace('\r', "\n");
    let normalized_body = body.replace("\r\n", "\n").replace('\r', "\n");
    let title = truncate_bytes(&normalized_title, 4096);
    let body = truncate_bytes(&normalized_body, MAX_TEXT_BYTES - title.len());
    if title.len() != normalized_title.len() || body.len() != normalized_body.len() {
        omissions.push(O::LocalTextLimit);
    }
    Ok((title.into(), body.into()))
}
pub(super) fn coverage(omissions: &[O]) -> SourceCoverage {
    if omissions.iter().any(|reason| {
        matches!(
            reason,
            O::RemoteDateUnavailable | O::LocalTextLimit | O::UnknownBlocks | O::ProviderTruncated
        )
    }) {
        SourceCoverage::Partial
    } else {
        SourceCoverage::Complete
    }
}
pub(super) fn finish(mut snapshot: ExistingToolSnapshot) -> Result<ExistingToolSnapshot, Failure> {
    snapshot.omission_reasons.sort_unstable();
    snapshot.omission_reasons.dedup();
    if serde_json::to_vec(&snapshot).map_err(|_| invalid())?.len() > MAX_SNAPSHOT_BYTES {
        return Err(Failure::new(Code::RemoteResponseTooLarge));
    }
    Ok(snapshot)
}
