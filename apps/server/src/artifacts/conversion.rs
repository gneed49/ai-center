//! Lossless bridge from an exact historical deliverable to a reviewable draft.
use super::{ArtifactDetail, CreateArtifact, SourceInput};
use crate::{
    error::{AppError, AppResult},
    idempotency::IdempotencyLease,
    service::AppState,
};
use serde_json::{Value, json};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct SourceDeliverable {
    id: i64,
    public_id: Uuid,
    deliverable_type: String,
    title: String,
    summary: String,
    content: Value,
    version: i32,
    status: String,
    content_hash: String,
}
/// Copies the selected immutable deliverable without generating or publishing anything.
/// # Errors
/// Rejects viewers, cross-project identities, unsupported contracts and oversized content.
pub async fn convert(
    state: &AppState,
    project: Uuid,
    deliverable: Uuid,
    lease: Option<&IdempotencyLease>,
) -> AppResult<ArtifactDetail> {
    super::editor(state)?;
    let mut tx = state.begin_request().await?;
    let project_id = super::project_id(&mut tx, project).await?;
    crate::company::data::require_active(&mut tx, project_id).await?;
    let item:SourceDeliverable=sqlx::query_as("select id,public_id,deliverable_type,title,summary,content,version,status,content_hash from app.deliverables where public_id=$1 and project_id=$2 and workspace_id=app.current_workspace_id()")
        .bind(deliverable).bind(project_id).fetch_optional(&mut *tx).await?.ok_or(AppError::NotFound)?;
    let artifact_type = match item.deliverable_type.as_str() {
        "feature-brief" => "specification",
        "technical-delivery-plan" => "technical_plan",
        _ => {
            return Err(AppError::Invalid(
                "Ce contrat de livrable ne dispose pas d’une conversion documentaire.".into(),
            ));
        }
    };
    let rows:Vec<(String,Uuid)>=sqlx::query_as("select distinct case source_kind when 'knowledge_entry_version' then 'knowledge' else 'context_pack' end,source_public_id from app.deliverable_sources where deliverable_id=$1 and workspace_id=app.current_workspace_id() and source_kind in ('knowledge_entry_version','context_pack')")
        .bind(item.id).fetch_all(&mut *tx).await?;
    let mut sources: Vec<SourceInput> = rows
        .into_iter()
        .map(|(kind, public_id)| SourceInput { kind, public_id })
        .collect();
    sources.push(SourceInput {
        kind: "deliverable".into(),
        public_id: item.public_id,
    });
    let body = format!("{}\n\n{}", item.summary, render(&item.content));
    let input = CreateArtifact {
        artifact_type: artifact_type.into(),
        title: item.title,
        body_markdown: body,
        structured_content: json!({"format":"converted-deliverable-v1",
            "origin":{"deliverable_id":item.public_id,"version":item.version,"status_at_capture":item.status,"content_hash":item.content_hash},
            "content":item.content}),
        sources,
    };
    let result = super::create_in_transaction(&mut tx, state, project, input, lease, None).await?;
    tx.commit().await?;
    Ok(result)
}
fn render(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Array(items) => items
            .iter()
            .map(|item| format!("- {}", render(item)))
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(items) => items
            .iter()
            .map(|(key, value)| format!("**{}**\n\n{}\n", key.replace('_', " "), render(value)))
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Null => "Non renseigné".into(),
        value => value.to_string(),
    }
}
