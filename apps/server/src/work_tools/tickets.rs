//! One immutable issue job per ticket; preparation is a read, admission is atomic.
use super::{
    Publication, publications, reliability,
    ticket_models::{
        PreviewTickets, PublishTickets, TicketCapacity, TicketCoverage, TicketCoverageItem,
        TicketCoverageQuery, TicketPreview, TicketPreviewItem, TicketPublicationResult,
    },
    ticket_projection,
};
use crate::{
    artifacts,
    error::{AppError, AppResult},
    idempotency::{self, IdempotencyLease},
    service::AppState,
};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

/// The qualified operation makes even an unfinished receipt artifact-specific.
#[must_use]
pub fn operation(artifact: Uuid) -> String {
    format!("publication.tickets.create:{artifact}")
}

pub fn canonicalize(input: &mut PreviewTickets) -> AppResult<()> {
    input.ticket_indexes = ticket_projection::indexes(&input.ticket_indexes)?;
    input.expected_target_id =
        ticket_projection::target(&input.expected_provider, &input.expected_target_id)?;
    Ok(())
}
pub fn canonicalize_command(input: &mut PublishTickets) -> AppResult<()> {
    let mut prepare = input.preparation();
    canonicalize(&mut prepare)?;
    input.ticket_indexes = prepare.ticket_indexes;
    input.expected_target_id = prepare.expected_target_id;
    Ok(())
}
#[derive(sqlx::FromRow)]
struct Version {
    id: i64,
    project_id: i64,
    project_public_id: Uuid,
    number: i32,
    kind: String,
    title: String,
    body_markdown: String,
    structured_content: Value,
    sources: Value,
    content_hash: String,
    status: String,
    current: bool,
}
async fn version(
    tx: &mut Transaction<'_, Postgres>,
    artifact: Uuid,
    selected: Uuid,
) -> AppResult<Version> {
    sqlx::query_as("select v.id,v.project_id,p.public_id as project_public_id,v.version as number,d.artifact_type as kind,v.title,v.body_markdown,v.structured_content,v.content_hash,v.status,d.current_version_id=v.id as current,
      coalesce((select jsonb_agg(s.snapshot order by s.source_kind,s.source_public_id) from app.artifact_version_sources s where s.version_id=v.id),'[]'::jsonb) as sources
      from app.artifact_documents d join app.artifact_document_versions v on v.document_id=d.id join app.projects p on p.id=d.project_id
      where d.public_id=$1 and v.public_id=$2 and d.workspace_id=app.current_workspace_id()")
        .bind(artifact).bind(selected).fetch_optional(&mut **tx).await?.ok_or(AppError::NotFound)
}
fn draft(version: &Version) -> AppResult<crate::artifacts::generation_contract::ArtifactDraft> {
    let draft = ticket_projection::draft(
        &version.kind,
        &version.structured_content,
        version
            .sources
            .as_array()
            .ok_or_else(|| AppError::Internal("Invalid artifact source snapshot".into()))?,
    )?;
    if draft.title != version.title
        || crate::artifacts::generation_contract::markdown(&draft) != version.body_markdown
    {
        return Err(AppError::Invalid(
            "Le contenu structuré ne correspond pas au document validé.".into(),
        ));
    }
    Ok(draft)
}
async fn current_jobs(
    tx: &mut Transaction<'_, Postgres>,
    version: i64,
    provider: &str,
    target: &str,
) -> AppResult<Vec<Publication>> {
    Ok(sqlx::query_as(&format!("{} where j.workspace_id=app.current_workspace_id() and j.artifact_version_id=$1 and j.provider=$2 and j.target_id=$3 and j.source_ticket_index>=0 order by j.source_ticket_index",publications::JOB_SELECT))
        .bind(version).bind(provider).bind(target).fetch_all(&mut **tx).await?)
}
/// Historical coverage is complete for the selected version, never a page of jobs.
pub async fn coverage(
    state: &AppState,
    artifact: Uuid,
    query: TicketCoverageQuery,
) -> AppResult<TicketCoverage> {
    let target = ticket_projection::target(&query.provider, &query.target_id)?;
    let mut tx = state.begin_request().await?;
    let version = version(&mut tx, artifact, query.version_id).await?;
    let draft = draft(&version)?;
    let jobs = current_jobs(&mut tx, version.id, &query.provider, &target).await?;
    let items = draft
        .tickets
        .into_iter()
        .zip(0_i16..)
        .map(|(ticket, index)| TicketCoverageItem {
            source_ticket_index: index,
            title: ticket.title,
            existing_publication: jobs
                .iter()
                .find(|j| j.source_ticket_index == index)
                .cloned(),
        })
        .collect();
    tx.commit().await?;
    Ok(TicketCoverage {
        artifact_id: artifact,
        source_version_number: version.number,
        version_id: query.version_id,
        provider: query.provider,
        target_id: target,
        items,
    })
}
struct Prepared {
    preview: TicketPreview,
    version: Version,
    connection: i64,
}
async fn prepare(
    tx: &mut Transaction<'_, Postgres>,
    state: &AppState,
    artifact: Uuid,
    input: PreviewTickets,
) -> AppResult<Prepared> {
    let version = version(tx, artifact, input.version_id).await?;
    if !version.current || version.status != "validated" {
        return Err(AppError::Conflict(
            "Validez et sélectionnez la version courante avant de publier.".into(),
        ));
    }
    crate::company::data::require_active(tx, version.project_id).await?;
    let draft = draft(&version)?;
    let destination:Option<(String,Option<String>)>=sqlx::query_as("select provider,target_id from app.artifact_destination_settings where workspace_id=app.current_workspace_id() and artifact_type=$1 and enabled and (project_id=$2 or project_id is null) order by project_id is not null desc limit 1")
        .bind(&version.kind).bind(version.project_id).fetch_optional(&mut **tx).await?;
    let (provider, target) = destination
        .and_then(|(p, t)| t.map(|t| (p, t)))
        .ok_or_else(|| AppError::Invalid("Configurez une destination pour les tickets.".into()))?;
    let target = ticket_projection::target(&provider, &target)?;
    if provider != input.expected_provider || target != input.expected_target_id {
        return Err(AppError::Conflict(
            "La destination a changé ; préparez à nouveau les tickets.".into(),
        ));
    }
    // Read authorization metadata under SELECT RLS: editors may publish but cannot
    // lock owner-only credential rows through UPDATE policies. Capture the revision;
    // the worker rechecks it before any HTTP and cancels a subsequently rotated job.
    let connection:Option<(i64,i32)>=sqlx::query_as("select id,revision from app.work_tool_connections where public_id=$1 and workspace_id=app.current_workspace_id() and provider=$2 and enabled")
        .bind(input.connection_id).bind(&provider).fetch_optional(&mut **tx).await?;
    let (connection, revision) = connection.ok_or_else(|| {
        AppError::Invalid("Choisissez une connexion active pour cette destination.".into())
    })?;
    let jobs = current_jobs(tx, version.id, &provider, &target).await?;
    // History is independent of current-ticket jobs: another identical batch
    // may arrive after preview and must be reused, not invalidate confirmation.
    let prior:Vec<Publication>=sqlx::query_as(&format!("{} where d.public_id=$1 and j.workspace_id=app.current_workspace_id() and j.provider=$2 and j.target_id=$3 and (j.artifact_version_id<>$4 or j.source_ticket_index=-1) order by j.public_id limit 501",publications::JOB_SELECT))
        .bind(artifact).bind(&provider).bind(&target).bind(version.id).fetch_all(&mut **tx).await?;
    if prior.len() > 500 {
        return Err(AppError::Invalid(
            "Plus de 500 publications existent pour cette destination ; une vérification de l’historique est nécessaire avant toute nouvelle publication.".into(),
        ));
    }
    let prior_fingerprint=idempotency::hash_request(&json!(prior.iter().map(|j|json!({"job":j.public_id,"version":j.version_id,"index":j.source_ticket_index,"status":j.status,"external_id":j.external_id,"external_url":j.external_url})).collect::<Vec<_>>()))?;
    let requires = prior
        .iter()
        .any(|j| !matches!(j.status.as_str(), "failed" | "cancelled"));
    let mut items = Vec::new();
    for index in &input.ticket_indexes {
        let (title, body) = ticket_projection::business_body(
            artifact,
            input.version_id,
            version.number,
            *index,
            &draft,
            version.sources.as_array().expect("validated sources"),
        )?;
        items.push(TicketPreviewItem {
            source_ticket_index: *index,
            title,
            business_body_markdown: body,
            existing_publication: jobs
                .iter()
                .find(|j| j.source_ticket_index == *index)
                .cloned(),
        });
    }
    let fingerprint = idempotency::hash_request(&json!({
        "format":"ticket-publication-preview-v1","actor_id":state.actor_id,"workspace_id":state.workspace_id,"project_id":version.project_public_id,
        "artifact_id":artifact,"version_id":input.version_id,"content_hash":version.content_hash,"ticket_indexes":input.ticket_indexes,
        "items":items.iter().map(|i|json!({"source_ticket_index":i.source_ticket_index,"title":i.title,"business_body_markdown":i.business_body_markdown})).collect::<Vec<_>>(),
        "provider":provider,"target_id":target,"connection_id":input.connection_id,"connection_revision":revision,"prior_publications_fingerprint":prior_fingerprint
    }))?;
    let usage = reliability::usage(tx).await?;
    let limits = reliability::limits()?;
    let existing_count = items
        .iter()
        .filter(|item| item.existing_publication.is_some())
        .count();
    let preview = TicketPreview {
        artifact_id: artifact,
        version_id: input.version_id,
        source_version_number: version.number,
        content_hash: version.content_hash.clone(),
        provider,
        target_id: target,
        connection_id: input.connection_id,
        connection_revision: revision,
        ticket_indexes: input.ticket_indexes,
        requested_count: items.len(),
        new_count: items.len() - existing_count,
        existing_count,
        items,
        prior_publications: prior,
        prior_publications_fingerprint: prior_fingerprint,
        requires_additional_confirmation: requires,
        preview_fingerprint: fingerprint,
        capacity: TicketCapacity {
            available_pending: (limits.max_pending_publications - usage.queued - usage.processing)
                .max(0),
            available_hourly: (limits.max_publications_per_hour - usage.publications_last_hour)
                .max(0),
            max_pending: limits.max_pending_publications,
            max_per_hour: limits.max_publications_per_hour,
        },
    };
    Ok(Prepared {
        preview,
        version,
        connection,
    })
}
async fn permitted(state: &AppState) -> AppResult<()> {
    artifacts::editor(state)?;
    super::cipher(state)?;
    if !crate::automation::enabled(state).await? {
        return Err(AppError::AutomationPaused);
    }
    Ok(())
}
pub async fn preview(
    state: &AppState,
    artifact: Uuid,
    mut input: PreviewTickets,
) -> AppResult<TicketPreview> {
    canonicalize(&mut input)?;
    permitted(state).await?;
    let mut tx = state.begin_request().await?;
    let prepared = prepare(&mut tx, state, artifact, input).await?;
    tx.commit().await?;
    Ok(prepared.preview)
}
pub async fn publish(
    state: &AppState,
    artifact: Uuid,
    mut input: PublishTickets,
    lease: Option<&IdempotencyLease>,
) -> AppResult<TicketPublicationResult> {
    canonicalize_command(&mut input)?;
    permitted(state).await?;
    let mut tx = state.begin_request().await?;
    // Same order as document save/publication: document, then project, then quota.
    sqlx::query("select id from app.artifact_documents where public_id=$1 and workspace_id=app.current_workspace_id() for update")
        .bind(artifact).fetch_optional(&mut *tx).await?.ok_or(AppError::NotFound)?;
    let Prepared {
        preview,
        version,
        connection,
    } = prepare(&mut tx, state, artifact, input.preparation()).await?;
    if preview.preview_fingerprint != input.preview_fingerprint
        || preview.prior_publications_fingerprint != input.prior_publications_fingerprint
    {
        return Err(AppError::Conflict(
            "L’aperçu a changé ; préparez et confirmez à nouveau les tickets.".into(),
        ));
    }
    if preview.new_count > 0
        && preview.requires_additional_confirmation
        && !input.confirm_additional_issues
    {
        return Err(AppError::Invalid(
            "Confirmez que ces tickets s’ajouteront aux publications antérieures.".into(),
        ));
    }
    // Pause and admission serialize under the same existing company lock.
    sqlx::query("select pg_advisory_xact_lock(hashtextextended('ai-automation:'||app.current_workspace_id()::text,0))").execute(&mut *tx).await?;
    let enabled:bool=sqlx::query_scalar("select coalesce((select enabled from app.workspace_automation_controls where workspace_id=app.current_workspace_id()),true)").fetch_one(&mut *tx).await?;
    if !enabled {
        return Err(AppError::AutomationPaused);
    }
    reliability::admit_publications(
        &mut tx,
        i64::try_from(preview.new_count)
            .map_err(|_| AppError::Internal("Invalid ticket count".into()))?,
    )
    .await?;
    let mut publications = Vec::new();
    for item in preview.items {
        if let Some(existing) = item.existing_publication {
            publications.push(existing);
            continue;
        }
        let id = Uuid::new_v4();
        let body = ticket_projection::final_body(&item.business_body_markdown, id);
        sqlx::query("insert into app.publication_jobs(public_id,workspace_id,project_id,artifact_version_id,connection_id,connection_revision,requested_by_actor_id,provider,target_id,title,body_markdown,content_hash,source_ticket_index) values($1,app.current_workspace_id(),$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)")
            .bind(id).bind(version.project_id).bind(version.id).bind(connection).bind(preview.connection_revision).bind(state.actor_id).bind(&preview.provider).bind(&preview.target_id).bind(&item.title).bind(body).bind(&version.content_hash).bind(item.source_ticket_index).execute(&mut *tx).await?;
        super::audit(&mut tx,state,"publication.queued",id,json!({"artifact_id":artifact,"version_id":input.version_id,"source_ticket_index":item.source_ticket_index,"provider":preview.provider,"target_id":preview.target_id})).await?;
        publications.push(publications::publication(&mut tx, id).await?);
    }
    super::audit(&mut tx,state,"publication.tickets.admitted",artifact,json!({"version_id":input.version_id,"ticket_indexes":input.ticket_indexes,"preview_fingerprint":input.preview_fingerprint,"confirm_additional_issues":input.confirm_additional_issues,"created_count":preview.new_count})).await?;
    let result = TicketPublicationResult {
        artifact_id: artifact,
        source_version_number: version.number,
        version_id: input.version_id,
        provider: preview.provider,
        target_id: preview.target_id,
        publications,
        created_count: preview.new_count,
        existing_count: preview.existing_count,
    };
    artifacts::complete(&mut tx, lease, &result).await?;
    tx.commit().await?;
    Ok(result)
}
/// Read only, including processing/expired receipts; never starts publication.
pub async fn receipt(state: &AppState, artifact: Uuid, key: Uuid) -> AppResult<Value> {
    let mut tx = state.begin_request().await?;
    let project:Option<(i64,String)>=sqlx::query_as("select d.project_id,p.status from app.artifact_documents d join app.projects p on p.id=d.project_id where d.public_id=$1 and d.workspace_id=app.current_workspace_id()")
        .bind(artifact).fetch_optional(&mut *tx).await?;
    let (project, status) = project.ok_or(AppError::NotFound)?;
    let result:Option<Value>=sqlx::query_scalar("select jsonb_build_object('status',case when expires_at<=now() then 'expired' when status='completed' then 'completed' when status='processing' and locked_until>now() then 'processing' when status='processing' then 'interrupted' when status='failed' and response_body->>'retryable'='true' then 'retryable' else 'failed' end,
       'can_retry',coalesce(expires_at>now() and ((status='processing' and locked_until<=now()) or (status='failed' and response_body->>'retryable'='true')),false), 'result',case when status='completed' then response_body else null end)
       from app.idempotency_records where workspace_id=app.current_workspace_id() and actor_id=app.current_actor_id() and project_id=$1 and operation_key=$2 and idempotency_key=$3")
        .bind(project).bind(operation(artifact)).bind(key.to_string()).fetch_optional(&mut *tx).await?;
    tx.commit().await?;
    let mut result =
        result.unwrap_or_else(|| json!({"status":"not_received","can_retry":true,"result":null}));
    if !matches!(state.workspace_role.as_str(), "owner" | "editor") || status != "active" {
        result["can_retry"] = json!(false);
    }
    Ok(result)
}
/// Resolve the public project before acquiring a command lease.
pub async fn project(state: &AppState, artifact: Uuid) -> AppResult<Uuid> {
    let mut tx = state.begin_request().await?;
    let project=sqlx::query_scalar("select p.public_id from app.artifact_documents d join app.projects p on p.id=d.project_id where d.public_id=$1 and d.workspace_id=app.current_workspace_id()")
        .bind(artifact).fetch_optional(&mut *tx).await?.ok_or(AppError::NotFound)?;
    tx.commit().await?;
    Ok(project)
}
