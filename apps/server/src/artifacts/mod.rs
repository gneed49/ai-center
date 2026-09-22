//! Manual, versioned company artifacts. No model or external publication is
//! invoked here; immutable versions preserve precisely what a person validated.
mod destinations;
pub mod models;
mod sources;
mod validation;

pub use destinations::{destinations, reset_destination, set_destination};
pub use models::*;

use std::fmt::Write as _;

use serde::Serialize;
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    idempotency::{self, IdempotencyLease},
    service::AppState,
};

const SUMMARY_SELECT: &str = "select d.public_id,p.public_id as project_id,d.artifact_type,
    v.title,v.status,v.public_id as current_version_id,v.version,d.created_at,d.updated_at
    from app.artifact_documents d join app.projects p on p.id=d.project_id
    join app.artifact_document_versions v on v.id=d.current_version_id";

pub(crate) fn editor(state: &AppState) -> AppResult<()> {
    if matches!(state.workspace_role.as_str(), "owner" | "editor") {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

pub(crate) async fn project_id(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> AppResult<i64> {
    sqlx::query_scalar("select id from app.projects where public_id=$1 and workspace_id=app.current_workspace_id()")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(AppError::NotFound)
}

pub(crate) async fn complete<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    lease: Option<&IdempotencyLease>,
    output: &T,
) -> AppResult<()> {
    if let Some(lease) = lease {
        idempotency::complete(
            tx,
            lease,
            200,
            serde_json::to_value(output).map_err(|error| AppError::Internal(error.to_string()))?,
        )
        .await?;
    }
    Ok(())
}

pub(crate) async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    project: Option<i64>,
    action: &str,
    object: Uuid,
    details: Value,
) -> AppResult<()> {
    sqlx::query("insert into app.audit_events(workspace_id,project_id,actor_id,action,object_kind,object_public_id,after_state)
        values(app.current_workspace_id(),$1,$2,$3,$4,$5,$6)")
        .bind(project).bind(actor).bind(action)
        .bind(if action.starts_with("artifact.destination") {"artifact_destination"} else {"artifact_document"})
        .bind(object).bind(details).execute(&mut **tx).await?;
    Ok(())
}

/// Lists the authorized project's current artifact versions.
///
/// # Errors
/// Returns invalid filters, an inaccessible project, or database failures.
pub async fn list(
    state: &AppState,
    project: Uuid,
    query: ListArtifacts,
) -> AppResult<Page<ArtifactSummary>> {
    let (limit, offset) = validation::pagination(&query)?;
    validation::filters(&query)?;
    let search = query.q.as_deref().unwrap_or("").trim();
    let mut tx = state.begin_request().await?;
    let project = project_id(&mut tx, project).await?;
    let filter=" where d.project_id=$1 and d.workspace_id=app.current_workspace_id()
        and ($2='' or strpos(lower(v.title),lower($2))>0 or strpos(lower(v.body_markdown),lower($2))>0)
        and ($3::text is null or d.artifact_type=$3) and ($4::text is null or v.status=$4)";
    let total:i64=sqlx::query_scalar(&format!("select count(*) from app.artifact_documents d join app.artifact_document_versions v on v.id=d.current_version_id{filter}"))
        .bind(project).bind(search).bind(&query.artifact_type).bind(&query.status).fetch_one(&mut *tx).await?;
    let items = sqlx::query_as(&format!(
        "{SUMMARY_SELECT}{filter} order by d.updated_at desc,d.id desc limit $5 offset $6"
    ))
    .bind(project)
    .bind(search)
    .bind(&query.artifact_type)
    .bind(&query.status)
    .bind(limit)
    .bind(offset)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Page {
        items,
        total,
        limit,
        offset,
    })
}

/// Reads an artifact and its canonical current version.
///
/// # Errors
/// Returns `NotFound` outside the active workspace, or a database failure.
pub async fn get(state: &AppState, artifact: Uuid) -> AppResult<ArtifactDetail> {
    let mut tx = state.begin_request().await?;
    let result = detail(&mut tx, artifact).await?;
    tx.commit().await?;
    Ok(result)
}

async fn detail(tx: &mut Transaction<'_, Postgres>, artifact: Uuid) -> AppResult<ArtifactDetail> {
    let artifact: ArtifactSummary = sqlx::query_as(&format!(
        "{SUMMARY_SELECT} where d.public_id=$1 and d.workspace_id=app.current_workspace_id()"
    ))
    .bind(artifact)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(AppError::NotFound)?;
    let current_version = read_version(tx, artifact.public_id, artifact.current_version_id).await?;
    Ok(ArtifactDetail {
        artifact,
        current_version,
    })
}

async fn read_version(
    tx: &mut Transaction<'_, Postgres>,
    artifact: Uuid,
    version: Uuid,
) -> AppResult<ArtifactVersion> {
    let body: Value = sqlx::query_scalar(
        "select jsonb_build_object('public_id',v.public_id,'version',v.version,
        'title',v.title,'body_markdown',v.body_markdown,'structured_content',v.structured_content,
        'status',v.status,'created_by_actor_id',v.created_by_actor_id,'created_at',v.created_at,
        'validated_at',v.validated_at,'content_hash',v.content_hash,
        'sources',coalesce((select jsonb_agg(s.snapshot order by s.source_kind,s.source_public_id)
            from app.artifact_version_sources s where s.version_id=v.id),'[]'::jsonb))
        from app.artifact_document_versions v join app.artifact_documents d on d.id=v.document_id
        where d.public_id=$1 and v.public_id=$2 and v.workspace_id=app.current_workspace_id()",
    )
    .bind(artifact)
    .bind(version)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(AppError::NotFound)?;
    serde_json::from_value(body).map_err(|error| AppError::Internal(error.to_string()))
}

/// Reads the exact historical version selected by the caller.
///
/// # Errors
/// Returns `NotFound` for inaccessible or mismatched identities, or a database failure.
pub async fn version(
    state: &AppState,
    artifact: Uuid,
    version: Uuid,
) -> AppResult<ArtifactVersion> {
    let mut tx = state.begin_request().await?;
    let result = read_version(&mut tx, artifact, version).await?;
    tx.commit().await?;
    Ok(result)
}

/// Lists immutable history in descending version order.
///
/// # Errors
/// Returns invalid pagination, inaccessible artifacts, or database failures.
pub async fn versions(
    state: &AppState,
    artifact: Uuid,
    query: ListArtifacts,
) -> AppResult<Page<ArtifactVersion>> {
    let (limit, offset) = validation::pagination(&query)?;
    if query.q.is_some() || query.artifact_type.is_some() || query.status.is_some() {
        return Err(AppError::Invalid(
            "Version history accepts only limit and offset".into(),
        ));
    }
    let mut tx = state.begin_request().await?;
    let document:i64=sqlx::query_scalar("select id from app.artifact_documents where public_id=$1 and workspace_id=app.current_workspace_id()")
        .bind(artifact).fetch_optional(&mut *tx).await?.ok_or(AppError::NotFound)?;
    let total = sqlx::query_scalar(
        "select count(*) from app.artifact_document_versions where document_id=$1",
    )
    .bind(document)
    .fetch_one(&mut *tx)
    .await?;
    let ids:Vec<Uuid>=sqlx::query_scalar("select public_id from app.artifact_document_versions where document_id=$1 order by version desc limit $2 offset $3")
        .bind(document).bind(limit).bind(offset).fetch_all(&mut *tx).await?;
    let mut items = Vec::with_capacity(ids.len());
    for id in ids {
        items.push(read_version(&mut tx, artifact, id).await?);
    }
    tx.commit().await?;
    Ok(Page {
        items,
        total,
        limit,
        offset,
    })
}

struct VersionWrite<'a> {
    document: i64,
    project: i64,
    number: i32,
    content: &'a CreateArtifact,
    status: &'a str,
    preserve_sources_from: Option<Uuid>,
}

async fn append(
    tx: &mut Transaction<'_, Postgres>,
    state: &AppState,
    input: VersionWrite<'_>,
) -> AppResult<Uuid> {
    let sources = if let Some(version) = input.preserve_sources_from {
        sources::from_version(tx, version).await?
    } else {
        sources::resolve(tx, &input.content.sources).await?
    };
    let content_hash = idempotency::hash_request(
        &json!({"title":input.content.title.trim(),"body_markdown":input.content.body_markdown,
        "structured_content":input.content.structured_content,"sources":sources.iter().map(|s| &s.snapshot).collect::<Vec<_>>()}),
    )?;
    let (id,public_id):(i64,Uuid)=sqlx::query_as("insert into app.artifact_document_versions
        (document_id,workspace_id,project_id,version,title,body_markdown,structured_content,status,created_by_actor_id,validated_at,content_hash)
        values($1,app.current_workspace_id(),$2,$3,$4,$5,$6,$7,$8,case when $7='validated' then now() else null end,$9)
        returning id,public_id")
        .bind(input.document).bind(input.project).bind(input.number).bind(input.content.title.trim())
        .bind(&input.content.body_markdown).bind(&input.content.structured_content).bind(input.status)
        .bind(state.actor_id).bind(content_hash).fetch_one(&mut **tx).await?;
    sources::store(tx, id, sources).await?;
    sqlx::query(
        "update app.artifact_documents set current_version_id=$2,updated_at=now() where id=$1",
    )
    .bind(input.document)
    .bind(id)
    .execute(&mut **tx)
    .await?;
    Ok(public_id)
}

async fn event(
    tx: &mut Transaction<'_, Postgres>,
    project: i64,
    artifact: Uuid,
    version: Uuid,
    action: &str,
) -> AppResult<()> {
    sqlx::query("insert into app.domain_events(workspace_id,project_id,event_type,aggregate_kind,aggregate_public_id,payload)
        values(app.current_workspace_id(),$1,$2,'artifact_document',$3,$4)")
        .bind(project).bind(action).bind(artifact).bind(json!({"artifact_id":artifact,"version_id":version}))
        .execute(&mut **tx).await?;
    Ok(())
}

/// Creates a manual draft and its first immutable version atomically.
///
/// # Errors
/// Rejects viewers, invalid content, unauthorized sources and database failures.
pub async fn create(
    state: &AppState,
    project: Uuid,
    input: CreateArtifact,
    lease: Option<&IdempotencyLease>,
) -> AppResult<ArtifactDetail> {
    editor(state)?;
    validation::content(&input)?;
    let mut tx = state.begin_request().await?;
    let project = project_id(&mut tx, project).await?;
    crate::company::data::require_active(&mut tx, project).await?;
    let (document,artifact):(i64,Uuid)=sqlx::query_as("insert into app.artifact_documents(workspace_id,project_id,artifact_type,created_by_actor_id)
        values(app.current_workspace_id(),$1,$2,$3) returning id,public_id")
        .bind(project).bind(&input.artifact_type).bind(state.actor_id).fetch_one(&mut *tx).await?;
    let version = append(
        &mut tx,
        state,
        VersionWrite {
            document,
            project,
            number: 1,
            content: &input,
            status: "draft",
            preserve_sources_from: None,
        },
    )
    .await?;
    audit(
        &mut tx,
        state.actor_id,
        Some(project),
        "artifact.created",
        artifact,
        json!({"version_id":version}),
    )
    .await?;
    let output = detail(&mut tx, artifact).await?;
    complete(&mut tx, lease, &output).await?;
    tx.commit().await?;
    Ok(output)
}

#[derive(sqlx::FromRow)]
struct Head {
    id: i64,
    project_id: i64,
    artifact_type: String,
    version_id: Uuid,
    version: i32,
    status: String,
}

async fn lock_head(
    tx: &mut Transaction<'_, Postgres>,
    artifact: Uuid,
    expected: Uuid,
) -> AppResult<Head> {
    // Lock the document first, then read its head in a fresh statement. Under
    // READ COMMITTED this observes a concurrent writer after waiting for it.
    let id:i64=sqlx::query_scalar("select id from app.artifact_documents where public_id=$1 and workspace_id=app.current_workspace_id() for update")
        .bind(artifact).fetch_optional(&mut **tx).await?.ok_or(AppError::NotFound)?;
    let head:Head=sqlx::query_as("select d.id,d.project_id,d.artifact_type,v.public_id as version_id,v.version,v.status
        from app.artifact_documents d join app.artifact_document_versions v on v.id=d.current_version_id where d.id=$1")
        .bind(id).fetch_one(&mut **tx).await?;
    if head.version_id != expected {
        return Err(AppError::Conflict(
            "This artifact changed. Reload its latest version before saving.".into(),
        ));
    }
    crate::company::data::require_active(tx, head.project_id).await?;
    Ok(head)
}

/// Appends a draft using an optimistic check against the document's current head.
///
/// # Errors
/// Rejects invalid content/sources, viewers, concurrent changes and database failures.
pub async fn save_draft(
    state: &AppState,
    artifact: Uuid,
    input: SaveDraft,
    lease: Option<&IdempotencyLease>,
) -> AppResult<ArtifactDetail> {
    editor(state)?;
    let mut tx = state.begin_request().await?;
    let head = lock_head(&mut tx, artifact, input.expected_version_id).await?;
    let content = CreateArtifact {
        artifact_type: head.artifact_type,
        title: input.title,
        body_markdown: input.body_markdown,
        structured_content: input.structured_content,
        sources: input.sources,
    };
    validation::content(&content)?;
    let version = append(
        &mut tx,
        state,
        VersionWrite {
            document: head.id,
            project: head.project_id,
            number: head.version + 1,
            content: &content,
            status: "draft",
            preserve_sources_from: None,
        },
    )
    .await?;
    audit(
        &mut tx,
        state.actor_id,
        Some(head.project_id),
        "artifact.revised",
        artifact,
        json!({"version_id":version,"previous_version_id":head.version_id}),
    )
    .await?;
    event(
        &mut tx,
        head.project_id,
        artifact,
        version,
        "artifact.revised",
    )
    .await?;
    let output = detail(&mut tx, artifact).await?;
    complete(&mut tx, lease, &output).await?;
    tx.commit().await?;
    Ok(output)
}

/// Appends a validated version without changing the draft's captured provenance.
///
/// # Errors
/// Rejects viewers, empty artifacts, stale expected heads and already validated versions.
pub async fn validate(
    state: &AppState,
    artifact: Uuid,
    input: ValidateArtifact,
    lease: Option<&IdempotencyLease>,
) -> AppResult<ArtifactDetail> {
    editor(state)?;
    let mut tx = state.begin_request().await?;
    let head = lock_head(&mut tx, artifact, input.expected_version_id).await?;
    if head.status != "draft" {
        return Err(AppError::Conflict(
            "This version is already validated".into(),
        ));
    }
    let previous = read_version(&mut tx, artifact, head.version_id).await?;
    if previous.body_markdown.trim().is_empty() && previous.structured_content == json!({}) {
        return Err(AppError::Invalid(
            "An empty artifact cannot be validated".into(),
        ));
    }
    let sources = previous
        .sources
        .iter()
        .map(|source| {
            serde_json::from_value(json!({"kind":source["kind"],"public_id":source["public_id"]}))
        })
        .collect::<Result<Vec<SourceInput>, _>>()
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let content = CreateArtifact {
        artifact_type: head.artifact_type,
        title: previous.title,
        body_markdown: previous.body_markdown,
        structured_content: previous.structured_content,
        sources,
    };
    let version = append(
        &mut tx,
        state,
        VersionWrite {
            document: head.id,
            project: head.project_id,
            number: head.version + 1,
            content: &content,
            status: "validated",
            preserve_sources_from: Some(head.version_id),
        },
    )
    .await?;
    audit(
        &mut tx,
        state.actor_id,
        Some(head.project_id),
        "artifact.validated",
        artifact,
        json!({"version_id":version,"previous_version_id":head.version_id}),
    )
    .await?;
    event(
        &mut tx,
        head.project_id,
        artifact,
        version,
        "artifact.validated",
    )
    .await?;
    let output = detail(&mut tx, artifact).await?;
    complete(&mut tx, lease, &output).await?;
    tx.commit().await?;
    Ok(output)
}

/// Export identity and provenance accompany the selected immutable version.
/// This is an internal export, never evidence of an external publication.
#[must_use]
pub fn markdown(artifact: Uuid, value: &ArtifactVersion) -> String {
    let mut output = format!(
        "# {}\n\nVersion : {} · {}\nArtefact : {}\nVersion immuable : {}\nEmpreinte : {}\n\n{}\n",
        value.title,
        value.version,
        value.status,
        artifact,
        value.public_id,
        value.content_hash,
        value.body_markdown
    );
    if value.structured_content != json!({}) {
        output.push_str("\n## Données structurées\n\n````json\n");
        output
            .push_str(&serde_json::to_string_pretty(&value.structured_content).unwrap_or_default());
        output.push_str("\n````\n");
    }
    output.push_str("\n## Sources de cette version\n\n");
    if value.sources.is_empty() {
        output.push_str("Aucune source rattachée ; contenu saisi manuellement.\n");
    }
    for source in &value.sources {
        let _ = writeln!(
            output,
            "- {} · {} · projet {} · version {}",
            source["kind"].as_str().unwrap_or(""),
            source["public_id"].as_str().unwrap_or(""),
            source["project_id"].as_str().unwrap_or(""),
            source["version"]
        );
    }
    output
}
