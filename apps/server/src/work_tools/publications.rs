use super::{
    audit,
    client::ToolClient,
    credential,
    models::{
        JobInput, ListPublications, Publication, PublicationDetail, Publications, PublishArtifact,
        ReconcilePublication,
    },
};
use crate::{
    artifacts::{self, complete, editor},
    error::{AppError, AppResult},
    idempotency::IdempotencyLease,
    service::AppState,
};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub(super) const JOB_SELECT: &str = "select j.source_ticket_index,j.title,v.version as source_version_number,j.public_id,d.public_id as artifact_id,v.public_id as version_id,c.public_id as connection_id,j.provider,j.target_id,j.status,j.external_id,j.external_url,j.error_code,j.attempt_count,j.created_at,j.updated_at from app.publication_jobs j join app.artifact_document_versions v on v.id=j.artifact_version_id join app.artifact_documents d on d.id=v.document_id join app.work_tool_connections c on c.id=j.connection_id";
pub(crate) fn marker(id: Uuid) -> String {
    format!("AI Center publication: {id}")
}
pub(crate) async fn job(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> AppResult<JobInput> {
    let job:JobInput=sqlx::query_as("select j.id,j.project_id,j.public_id,c.public_id as connection_public_id,j.connection_revision,j.provider,j.target_id,j.title,j.body_markdown,j.status,j.external_id,j.external_url from app.publication_jobs j join app.work_tool_connections c on c.id=j.connection_id where j.public_id=$1 and j.workspace_id=app.current_workspace_id()")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(AppError::NotFound)?;
    crate::company::data::require_active(tx, job.project_id).await?;
    Ok(job)
}
pub(super) async fn publication(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> AppResult<Publication> {
    sqlx::query_as(&format!(
        "{JOB_SELECT} where j.public_id=$1 and j.workspace_id=app.current_workspace_id()"
    ))
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(AppError::NotFound)
}
pub async fn list(
    state: &AppState,
    artifact: Uuid,
    query: ListPublications,
) -> AppResult<Publications> {
    let limit = query.limit.unwrap_or(25);
    let offset = query.offset.unwrap_or(0);
    if !(1..=100).contains(&limit) || !(0..=1_000_000).contains(&offset) {
        return Err(AppError::Invalid("Invalid publication pagination".into()));
    }
    let mut tx = state.begin_request().await?;
    let exists:bool=sqlx::query_scalar("select exists(select 1 from app.artifact_documents where public_id=$1 and workspace_id=app.current_workspace_id())")
        .bind(artifact).fetch_one(&mut *tx).await?;
    if !exists {
        return Err(AppError::NotFound);
    }
    let items=sqlx::query_as(&format!("{JOB_SELECT} where d.public_id=$1 and j.workspace_id=app.current_workspace_id() order by j.created_at desc,j.id desc limit $2 offset $3"))
        .bind(artifact).bind(limit).bind(offset).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(Publications {
        items,
        limit,
        offset,
    })
}
pub async fn detail(state: &AppState, id: Uuid) -> AppResult<PublicationDetail> {
    let mut tx = state.begin_request().await?;
    let publication = publication(&mut tx, id).await?;
    let observations=sqlx::query_as("select o.public_id,o.observed_at,o.observation_kind,o.external_id,o.external_url,o.remote_updated_at,o.snapshot from app.publication_observations o join app.publication_jobs j on j.id=o.publication_job_id where j.public_id=$1 and j.workspace_id=app.current_workspace_id() order by o.id desc limit 50")
        .bind(id).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(PublicationDetail {
        publication,
        observations,
    })
}
pub async fn publish(
    state: &AppState,
    artifact: Uuid,
    input: PublishArtifact,
    lease: Option<&IdempotencyLease>,
) -> AppResult<Publication> {
    editor(state)?;
    if !crate::automation::enabled(state).await? {
        return Err(AppError::AutomationPaused);
    }
    super::cipher(state)?;
    let version = artifacts::version(state, artifact, input.version_id).await?;
    let job_id = Uuid::new_v4();
    let body = format!(
        "{}\n\n---\n{}\n",
        artifacts::markdown(artifact, &version),
        marker(job_id)
    );
    if body.len() > 61_440 {
        return Err(AppError::Invalid("This artifact exceeds the 60 KiB publication limit; export it or split it into smaller documents".into()));
    }
    let mut tx = state.begin_request().await?;
    // Lock against a concurrent draft save/validation, then inspect the fresh head.
    let document:Option<(i64,i64,String)>=sqlx::query_as("select id,project_id,artifact_type from app.artifact_documents where public_id=$1 and workspace_id=app.current_workspace_id() for update")
        .bind(artifact).fetch_optional(&mut *tx).await?;
    let (document, project, kind) = document.ok_or(AppError::NotFound)?;
    crate::company::data::require_active(&mut tx, project).await?;
    let selected:Option<(i64,String,String)>=sqlx::query_as("select v.id,v.status,v.content_hash from app.artifact_documents d join app.artifact_document_versions v on v.id=d.current_version_id where d.id=$1 and v.public_id=$2")
        .bind(document).bind(input.version_id).fetch_optional(&mut *tx).await?;
    let (version_id, status, hash) = selected.ok_or_else(|| {
        AppError::Conflict("The artifact has a newer version; reload before publishing".into())
    })?;
    if status != "validated" {
        return Err(AppError::Invalid(
            "Explicitly validate this artifact version before publishing".into(),
        ));
    }
    let destination:Option<(String,Option<String>)>=sqlx::query_as("select provider,target_id from app.artifact_destination_settings where workspace_id=app.current_workspace_id() and artifact_type=$1 and enabled and (project_id=$2 or project_id is null) order by project_id is not null desc limit 1")
        .bind(&kind).bind(project).fetch_optional(&mut *tx).await?;
    let (provider,target)=destination.filter(|row|row.0!="internal").and_then(|row|row.1.map(|target|(row.0,target)))
        .ok_or_else(||AppError::Invalid("Configure an external destination before publishing; internal artifacts can be exported directly".into()))?;
    if provider != input.expected_provider || target != input.expected_target_id {
        return Err(AppError::Conflict(
            "The publication destination changed; review it again".into(),
        ));
    }
    // Destination settings may contain a valid compact UUID or repository case.
    // Canonicalize the captured identity before uniqueness and provider checks.
    let target = if provider == "github" {
        target.to_ascii_lowercase()
    } else {
        Uuid::parse_str(&target)
            .map_err(|_| AppError::Invalid("Invalid destination identity".into()))?
            .to_string()
    };
    let connection:Option<(i64,i32)>=sqlx::query_as("select id,revision from app.work_tool_connections where public_id=$1 and workspace_id=app.current_workspace_id() and provider=$2 and enabled")
        .bind(input.connection_id).bind(&provider).fetch_optional(&mut *tx).await?;
    let (connection, revision) = connection.ok_or_else(|| {
        AppError::Invalid("Choose an enabled connection for this destination".into())
    })?;
    let existing:Option<Uuid>=sqlx::query_scalar("select public_id from app.publication_jobs where workspace_id=app.current_workspace_id() and artifact_version_id=$1 and provider=$2 and target_id=$3 and source_ticket_index=-1")
        .bind(version_id).bind(&provider).bind(&target).fetch_optional(&mut *tx).await?;
    let id = if let Some(existing) = existing {
        existing
    } else {
        if matches!(kind.as_str(), "product_tickets" | "technical_tickets")
            && matches!(provider.as_str(), "linear" | "github")
        {
            return Err(AppError::Invalid(
                "Préparez et confirmez les tickets distincts avant leur publication.".into(),
            ));
        }
        super::reliability::admit_publication(&mut tx).await?;
        sqlx::query("insert into app.publication_jobs(public_id,workspace_id,project_id,artifact_version_id,connection_id,connection_revision,requested_by_actor_id,provider,target_id,title,body_markdown,content_hash) values($1,app.current_workspace_id(),$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)")
            .bind(job_id).bind(project).bind(version_id).bind(connection).bind(revision).bind(state.actor_id).bind(&provider).bind(&target).bind(&version.title).bind(body).bind(hash).execute(&mut *tx).await?;
        audit(&mut tx,state,"publication.queued",job_id,json!({"artifact_id":artifact,"version_id":input.version_id,"provider":provider,"target_id":target})).await?;
        job_id
    };
    let result = publication(&mut tx, id).await?;
    complete(&mut tx, lease, &result).await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn reconcile(
    state: &AppState,
    id: Uuid,
    input: ReconcilePublication,
    lease: Option<&IdempotencyLease>,
) -> AppResult<Publication> {
    observe(
        state,
        id,
        Some(input.external_id),
        lease,
        &ToolClient::official()?,
    )
    .await
}
pub async fn refresh(
    state: &AppState,
    id: Uuid,
    lease: Option<&IdempotencyLease>,
) -> AppResult<Publication> {
    observe(state, id, None, lease, &ToolClient::official()?).await
}
pub(crate) async fn observe(
    state: &AppState,
    id: Uuid,
    reconcile_id: Option<String>,
    lease: Option<&IdempotencyLease>,
    client: &ToolClient,
) -> AppResult<Publication> {
    editor(state)?;
    let mut tx = state.begin_request().await?;
    let before = job(&mut tx, id).await?;
    tx.commit().await?;
    let reconciling = reconcile_id.is_some();
    if (reconciling && before.status != "needs_review")
        || (!reconciling
            && !matches!(
                before.status.as_str(),
                "succeeded" | "conflict" | "unavailable"
            ))
    {
        return Err(AppError::Conflict(
            "The publication state does not permit this operation".into(),
        ));
    }
    let external_id = reconcile_id
        .or_else(|| before.external_id.clone())
        .ok_or(AppError::NotFound)?;
    let normalized = super::client::normalize_external_id(&before.provider, &external_id)
        .ok_or_else(|| {
            AppError::Invalid(
                "Use the object UUID for Notion/Linear, or the issue number for GitHub".into(),
            )
        })?;
    let credential = credential(state, before.connection_public_id).await?;
    super::reliability::admit_read(state, id).await?;
    let remote = client
        .read(
            &before.provider,
            &credential.secret,
            &before.target_id,
            &normalized,
        )
        .await;
    let mut tx = state.begin_request().await?;
    sqlx::query("select id from app.publication_jobs where public_id=$1 and workspace_id=app.current_workspace_id() for update")
        .bind(id).fetch_optional(&mut *tx).await?.ok_or(AppError::NotFound)?;
    let current = job(&mut tx, id).await?;
    if current.status != before.status || current.external_id != before.external_id {
        return Err(AppError::Conflict(
            "The publication changed during verification; reload it".into(),
        ));
    }
    let (new_status, error, observation) = match remote {
        Ok(receipt) => {
            if receipt.target_id != before.target_id
                || receipt.external_id != normalized
                || !receipt.complete
                || !receipt.body_markdown.contains(&marker(id))
            {
                return Err(AppError::Conflict("The remote object cannot be completely verified against this publication marker and destination".into()));
            }
            let latest:Option<Value>=sqlx::query_scalar("select snapshot from app.publication_observations where publication_job_id=$1 and observation_kind<>'unavailable' order by id asc limit 1")
                .bind(before.id).fetch_optional(&mut *tx).await?;
            let changed = latest.as_ref().map_or_else(
                || {
                    receipt.title != before.title
                        || receipt.body_markdown.trim() != before.body_markdown.trim()
                },
                |snapshot| {
                    snapshot["title"] != receipt.title
                        || snapshot["body_markdown"] != receipt.body_markdown
                },
            );
            let kind = if reconciling {
                "reconciled"
            } else if changed {
                "changed"
            } else {
                "unchanged"
            };
            let status = if changed { "conflict" } else { "succeeded" };
            (status, None, Some((kind, receipt)))
        }
        Err(error) => {
            if reconciling {
                return Err(AppError::Invalid(format!(
                    "Remote verification did not succeed ({})",
                    error.code
                )));
            }
            ("unavailable", Some(error.code), None)
        }
    };
    let (remote_id, url) = if let Some((kind, receipt)) = observation {
        sqlx::query("insert into app.publication_observations(workspace_id,publication_job_id,observation_kind,external_id,external_url,remote_updated_at,snapshot) values(app.current_workspace_id(),$1,$2,$3,$4,$5,$6)")
            .bind(before.id).bind(kind).bind(&receipt.external_id).bind(&receipt.external_url).bind(&receipt.remote_updated_at).bind(json!(receipt)).execute(&mut *tx).await?;
        (receipt.external_id, receipt.external_url)
    } else {
        let remote_id = before.external_id.ok_or(AppError::NotFound)?;
        let url = before.external_url.ok_or(AppError::NotFound)?;
        sqlx::query("insert into app.publication_observations(workspace_id,publication_job_id,observation_kind,external_id,external_url,snapshot) values(app.current_workspace_id(),$1,'unavailable',$2,$3,$4)")
            .bind(before.id).bind(&remote_id).bind(&url).bind(json!({"complete":false,"error_code":error})).execute(&mut *tx).await?;
        (remote_id, url)
    };
    sqlx::query("update app.publication_jobs set status=$2,error_code=$3,external_id=$4,external_url=$5,updated_at=now() where id=$1")
        .bind(before.id).bind(new_status).bind(error).bind(remote_id).bind(url).execute(&mut *tx).await?;
    audit(
        &mut tx,
        state,
        if reconciling {
            "publication.reconciled"
        } else {
            "publication.refreshed"
        },
        id,
        json!({"status":new_status}),
    )
    .await?;
    let result = publication(&mut tx, id).await?;
    complete(&mut tx, lease, &result).await?;
    tx.commit().await?;
    Ok(result)
}

/// Cancels a queued publication atomically against the worker claim.
/// Processing and ambiguous creates must preserve their actual remote outcome.
pub async fn cancel(
    state: &AppState,
    id: Uuid,
    lease: Option<&IdempotencyLease>,
) -> AppResult<Publication> {
    editor(state)?;
    let mut tx = state.begin_request().await?;
    let status:Option<String>=sqlx::query_scalar("select status from app.publication_jobs where public_id=$1 and workspace_id=app.current_workspace_id() for update")
        .bind(id).fetch_optional(&mut *tx).await?;
    match status.as_deref(){
        Some("queued")=>{
            sqlx::query("update app.publication_jobs set status='cancelled',error_code='cancelled_by_user',updated_at=now() where public_id=$1 and workspace_id=app.current_workspace_id()")
                .bind(id).execute(&mut *tx).await?;
            audit(&mut tx,state,"publication.cancelled",id,json!({})).await?;
        },
        Some("cancelled")=>{},None=>return Err(AppError::NotFound),
        _=>return Err(AppError::Conflict("This publication has already started; its external result must be verified before any further action".into())),
    }
    let result = publication(&mut tx, id).await?;
    complete(&mut tx, lease, &result).await?;
    tx.commit().await?;
    Ok(result)
}
