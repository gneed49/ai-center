use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};

use super::models::SourceInput;
use crate::error::{AppError, AppResult};

#[derive(sqlx::FromRow)]
pub(super) struct ResolvedSource {
    id: i64,
    project_id: i64,
    kind: String,
    pub snapshot: Value,
}

pub(super) async fn resolve(
    tx: &mut Transaction<'_, Postgres>,
    inputs: &[SourceInput],
) -> AppResult<Vec<ResolvedSource>> {
    let mut result = Vec::with_capacity(inputs.len());
    for input in inputs {
        let sql = match input.kind.as_str() {
            "knowledge" => {
                "select s.id,s.project_id,jsonb_build_object('public_id',s.public_id,'project_id',p.public_id,'title',s.title,'version',s.version_number,'status_at_capture',s.status) as snapshot from app.knowledge_entry_versions s join app.projects p on p.id=s.project_id where s.public_id=$1 and s.workspace_id=app.current_workspace_id()"
            }
            "context_pack" => {
                "select s.id,s.project_id,jsonb_build_object('public_id',s.public_id,'project_id',p.public_id,'title',s.objective,'version',s.version,'status_at_capture',s.status,'content_hash',s.content_hash) as snapshot from app.context_packs s join app.projects p on p.id=s.project_id where s.public_id=$1 and s.workspace_id=app.current_workspace_id()"
            }
            "deliverable" => {
                "select s.id,s.project_id,jsonb_build_object('public_id',s.public_id,'project_id',p.public_id,'title',s.title,'version',s.version,'status_at_capture',s.status,'content_hash',s.content_hash) as snapshot from app.deliverables s join app.projects p on p.id=s.project_id where s.public_id=$1 and s.workspace_id=app.current_workspace_id()"
            }
            "artifact_version" => {
                "select s.id,s.project_id,jsonb_build_object('public_id',s.public_id,'project_id',p.public_id,'artifact_id',d.public_id,'title',s.title,'version',s.version,'status_at_capture',s.status,'content_hash',s.content_hash) as snapshot from app.artifact_document_versions s join app.artifact_documents d on d.id=s.document_id join app.projects p on p.id=s.project_id where s.public_id=$1 and s.workspace_id=app.current_workspace_id()"
            }
            "session" => {
                "select s.id,s.project_id,jsonb_build_object('public_id',s.public_id,'project_id',p.public_id,'title',s.title,'version',null,'origin_only',true,'scope',n.node_key,'captured_at',now(),'message_count',(select count(*) from app.messages m where m.session_id=s.id),'last_message_id',(select m.public_id from app.messages m where m.session_id=s.id order by m.created_at desc,m.id desc limit 1),'last_message_at',(select m.created_at from app.messages m where m.session_id=s.id order by m.created_at desc,m.id desc limit 1),'last_user_message_id',(select m.public_id from app.messages m where m.session_id=s.id and m.role='user' order by m.created_at desc,m.id desc limit 1),'last_assistant_message_id',(select m.public_id from app.messages m where m.session_id=s.id and m.role='assistant' order by m.created_at desc,m.id desc limit 1)) as snapshot from app.sessions s join app.projects p on p.id=s.project_id join app.context_nodes n on n.id=s.context_node_id where s.public_id=$1 and s.workspace_id=app.current_workspace_id()"
            }
            _ => return Err(AppError::Invalid("Unknown artifact source kind".into())),
        };
        let (id, project_id, mut snapshot): (i64, i64, Value) = sqlx::query_as(sql)
            .bind(input.public_id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or(AppError::NotFound)?;
        snapshot["kind"] = json!(input.kind);
        result.push(ResolvedSource {
            id,
            project_id,
            kind: input.kind.clone(),
            snapshot,
        });
    }
    // Canonical ordering makes the digest independent from input ordering.
    result.sort_by(|a, b| {
        a.kind.cmp(&b.kind).then_with(|| {
            a.snapshot["public_id"]
                .as_str()
                .cmp(&b.snapshot["public_id"].as_str())
        })
    });
    Ok(result)
}

pub(super) async fn store(
    tx: &mut Transaction<'_, Postgres>,
    version: i64,
    sources: Vec<ResolvedSource>,
) -> AppResult<()> {
    for source in sources {
        let source_public_id = source.snapshot["public_id"]
            .as_str()
            .ok_or_else(|| AppError::Internal("source identity missing".into()))?
            .parse::<uuid::Uuid>()
            .map_err(|error| AppError::Internal(error.to_string()))?;
        sqlx::query("insert into app.artifact_version_sources(workspace_id,version_id,source_project_id,source_kind,
            knowledge_version_id,context_pack_id,deliverable_id,session_id,source_artifact_version_id,source_public_id,snapshot)
            values(app.current_workspace_id(),$1,$2,$3,$4,$5,$6,$7,$8,$9,$10)")
            .bind(version).bind(source.project_id).bind(&source.kind)
            .bind((source.kind=="knowledge").then_some(source.id))
            .bind((source.kind=="context_pack").then_some(source.id))
            .bind((source.kind=="deliverable").then_some(source.id))
            .bind((source.kind=="session").then_some(source.id))
            .bind((source.kind=="artifact_version").then_some(source.id))
            .bind(source_public_id).bind(source.snapshot).execute(&mut **tx).await?;
    }
    Ok(())
}

/// Validation preserves the captured origin instead of silently refreshing a
/// conversation that may have received more messages since the draft was saved.
pub(super) async fn from_version(
    tx: &mut Transaction<'_, Postgres>,
    version: uuid::Uuid,
) -> AppResult<Vec<ResolvedSource>> {
    Ok(sqlx::query_as("select coalesce(s.knowledge_version_id,s.context_pack_id,s.deliverable_id,s.session_id,s.source_artifact_version_id) as id,
        s.source_project_id as project_id,s.source_kind as kind,s.snapshot
        from app.artifact_version_sources s join app.artifact_document_versions v on v.id=s.version_id
        where v.public_id=$1 and s.workspace_id=app.current_workspace_id() order by s.source_kind,s.source_public_id")
        .bind(version).fetch_all(&mut **tx).await?)
}
