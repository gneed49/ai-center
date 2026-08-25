//! Read-only external-reference orchestration.
//!
//! GitHub remains the canonical system. This module only observes an
//! allowlisted pull-request projection, stores append-only observations, and
//! lets a human promote an observed reference from candidate evidence to valid
//! evidence. No code path in this module calls a GitHub write API.

#![allow(clippy::too_many_lines)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    auth::RequestContext,
    error::{AppError, AppResult},
    idempotency::{self, BeginOutcome, BeginRequest, FailureDisposition, IdempotencyLease},
    integrations::{
        GitHubRuntime,
        github::{
            GitHubCheckObservation, GitHubObserveResult, GitHubPullRequestCursor,
            GitHubPullRequestIdentity, GitHubPullRequestObservation,
        },
    },
    service::AppState,
};

const EVIDENCE_TYPE: &str = "github_pull_request";
const CREATE_OPERATION: &str = "external_reference.github_pr.create";
const REFRESH_OPERATION: &str = "external_reference.github_pr.refresh";
const EVIDENCE_CREATE_OPERATION: &str = "external_reference.evidence.create";
const EVIDENCE_REVIEW_OPERATION: &str = "external_reference.evidence.review";

/// Request body for tracking one canonical GitHub pull request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreatePullRequestReference {
    pub url: String,
    /// Optional explicit connection selection. When omitted, the active
    /// connection matching the configured GitHub App installation is used.
    #[serde(default)]
    pub tool_connection_id: Option<Uuid>,
}

/// A project-scoped external reference suitable for collection views.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExternalReferenceSummary {
    pub public_id: Uuid,
    pub project_public_id: Uuid,
    pub tool_connection_public_id: Uuid,
    pub provider: String,
    pub object_kind: String,
    pub external_id: String,
    pub canonical_url: String,
    pub repository_full_name: String,
    pub display_title: String,
    pub sync_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error_code: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Immutable observation of the allowlisted GitHub pull-request state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExternalReferenceObservationView {
    pub public_id: Uuid,
    pub observation_status: String,
    pub content_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    pub observed_state: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_updated_at: Option<DateTime<Utc>>,
    pub observed_at: DateTime<Utc>,
}

/// Evidence derived from an external reference. `valid` is reachable only
/// through [`review_evidence`] with an explicit human `validate` decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExternalEvidenceView {
    pub public_id: Uuid,
    pub requirement_public_id: Uuid,
    pub requirement_version_public_id: Uuid,
    pub deliverable_public_id: Uuid,
    pub deliverable_section_public_id: Uuid,
    pub evidence_type: String,
    pub title: String,
    pub description: String,
    pub source_reference: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// Detail projection returned by create, get, and refresh operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExternalReferenceView {
    #[serde(flatten)]
    pub reference: ExternalReferenceSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_observation: Option<ExternalReferenceObservationView>,
    pub evidences: Vec<ExternalEvidenceView>,
}

/// Creates only a candidate. Human review is deliberately a separate command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateExternalEvidence {
    pub requirement_id: Uuid,
    pub deliverable_id: Uuid,
    pub deliverable_section_id: Uuid,
    pub title: String,
    #[serde(default)]
    pub description: String,
}

/// Explicit human decision for an existing candidate evidence.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceDecision {
    Validate,
    Reject,
}

/// Review command kept separate from candidate creation so `valid` can never
/// be a provider-supplied state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewExternalEvidence {
    pub decision: EvidenceDecision,
}

#[derive(Debug, sqlx::FromRow)]
struct ProjectScope {
    project_id: i64,
    workspace_id: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct ConnectionRecord {
    id: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ReferenceRecord {
    id: i64,
    workspace_id: i64,
    project_id: i64,
    public_id: Uuid,
    project_public_id: Uuid,
    tool_connection_public_id: Uuid,
    provider: String,
    object_kind: String,
    external_id: String,
    canonical_url: String,
    repository_full_name: String,
    display_title: String,
    sync_status: String,
    etag: Option<String>,
    last_synced_at: Option<DateTime<Utc>>,
    last_error_code: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ReferenceRecord {
    fn summary(self) -> ExternalReferenceSummary {
        ExternalReferenceSummary {
            public_id: self.public_id,
            project_public_id: self.project_public_id,
            tool_connection_public_id: self.tool_connection_public_id,
            provider: self.provider,
            object_kind: self.object_kind,
            external_id: self.external_id,
            canonical_url: self.canonical_url,
            repository_full_name: self.repository_full_name,
            display_title: self.display_title,
            sync_status: self.sync_status,
            etag: self.etag,
            last_synced_at: self.last_synced_at,
            last_error_code: self.last_error_code,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ObservationRecord {
    public_id: Uuid,
    observation_status: String,
    content_hash: String,
    etag: Option<String>,
    observed_state: Value,
    provider_updated_at: Option<DateTime<Utc>>,
    observed_at: DateTime<Utc>,
}

impl ObservationRecord {
    fn view(self) -> ExternalReferenceObservationView {
        ExternalReferenceObservationView {
            public_id: self.public_id,
            observation_status: self.observation_status,
            content_hash: self.content_hash,
            etag: self.etag,
            observed_state: self.observed_state,
            provider_updated_at: self.provider_updated_at,
            observed_at: self.observed_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct EvidenceRecord {
    public_id: Uuid,
    requirement_public_id: Uuid,
    requirement_version_public_id: Uuid,
    deliverable_public_id: Uuid,
    deliverable_section_public_id: Uuid,
    evidence_type: String,
    title: String,
    description: String,
    source_reference: String,
    status: String,
    created_at: DateTime<Utc>,
}

impl EvidenceRecord {
    fn view(self) -> ExternalEvidenceView {
        ExternalEvidenceView {
            public_id: self.public_id,
            requirement_public_id: self.requirement_public_id,
            requirement_version_public_id: self.requirement_version_public_id,
            deliverable_public_id: self.deliverable_public_id,
            deliverable_section_public_id: self.deliverable_section_public_id,
            evidence_type: self.evidence_type,
            title: self.title,
            description: self.description,
            source_reference: self.source_reference,
            status: self.status,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct EvidenceIdentifiers {
    id: i64,
    workspace_id: i64,
    project_id: i64,
    requirement_entry_id: i64,
    requirement_version_id: i64,
    deliverable_id: Option<i64>,
    deliverable_section_id: Option<i64>,
    source_reference: String,
    status: String,
}

#[derive(Debug, sqlx::FromRow)]
struct PreviousReferenceState {
    head_sha: Option<String>,
    observation_status: Option<String>,
    observation_hash: Option<String>,
}

/// Lists GitHub references belonging to exactly one project and workspace.
///
/// # Errors
///
/// Returns a database error when the scoped collection cannot be read.
pub async fn list(
    state: &AppState,
    context: &RequestContext,
    project_public_id: Uuid,
) -> AppResult<Vec<ExternalReferenceSummary>> {
    let mut tx = begin_scoped_transaction(state, context).await?;
    let rows = sqlx::query_as::<_, ReferenceRecord>(REFERENCE_SELECT_BY_PROJECT)
        .bind(state.workspace_id)
        .bind(project_public_id)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(rows.into_iter().map(ReferenceRecord::summary).collect())
}

/// Creates or replays a project-scoped GitHub pull-request reference.
///
/// The durable idempotency claim and preflight lookup are committed before the
/// GitHub request. The provider call therefore never holds a database lock or
/// transaction open.
///
/// # Errors
///
/// Returns a validation, scope, idempotency, connector, or database error when
/// the import cannot be completed durably.
pub async fn create_pull_request(
    state: &AppState,
    context: &RequestContext,
    github: &GitHubRuntime,
    project_public_id: Uuid,
    idempotency_key: Uuid,
    input: CreatePullRequestReference,
) -> AppResult<ExternalReferenceView> {
    let identity = GitHubPullRequestIdentity::parse(input.url.trim())?;
    let normalized_input = CreatePullRequestReference {
        url: identity.canonical_url(),
        tool_connection_id: input.tool_connection_id,
    };
    let request_hash = idempotency::hash_request(&normalized_input)?;

    let mut claim_tx = begin_scoped_transaction(state, context).await?;
    let scope = resolve_project(&mut claim_tx, state, project_public_id).await?;
    let connection = resolve_connection(
        &mut claim_tx,
        scope.workspace_id,
        github.installation_id,
        normalized_input.tool_connection_id,
    )
    .await?;
    let lease = match idempotency::begin(
        &mut claim_tx,
        BeginRequest {
            workspace_id: scope.workspace_id,
            project_id: Some(scope.project_id),
            actor_id: state.actor_id,
            operation_key: CREATE_OPERATION,
            idempotency_key: &idempotency_key.to_string(),
            request_hash: &request_hash,
        },
    )
    .await?
    {
        BeginOutcome::New { lease, .. } => lease,
        BeginOutcome::InProgress { .. } => {
            return Err(AppError::Conflict(
                "this GitHub reference import is already in progress; retry shortly".into(),
            ));
        }
        BeginOutcome::Replay { response, .. } => {
            claim_tx.commit().await?;
            return replay(response);
        }
    };
    claim_tx.commit().await?;

    let provider_result = github
        .client
        .observe_pull_request(identity.clone(), None)
        .await;
    let (observation, etag) = match provider_result {
        Ok(GitHubObserveResult::Observed { observation, etag }) => (*observation, etag),
        Ok(GitHubObserveResult::NotModified) => {
            let error = AppError::Connector(
                "GitHub unexpectedly returned not-modified for a new import".into(),
            );
            record_provider_failure(state, context, &lease, &error).await?;
            return Err(error);
        }
        Err(error) => {
            record_provider_failure(state, context, &lease, &error).await?;
            return Err(error);
        }
    };
    if let Err(error) = validate_observation(&identity, &observation) {
        record_provider_failure(state, context, &lease, &error).await?;
        return Err(error);
    }

    let mut tx = begin_scoped_transaction(state, context).await?;
    let view = persist_observed_reference(
        &mut tx,
        state,
        scope,
        connection,
        &identity,
        &observation,
        etag,
    )
    .await?;
    complete(&mut tx, &lease, &view).await?;
    tx.commit().await?;
    Ok(view)
}

/// Loads one reference only when it belongs to the selected workspace.
///
/// # Errors
///
/// Returns `NotFound` outside the workspace and a database error on read
/// failure.
pub async fn get(
    state: &AppState,
    context: &RequestContext,
    reference_public_id: Uuid,
) -> AppResult<ExternalReferenceView> {
    let mut tx = begin_scoped_transaction(state, context).await?;
    let reference = load_reference(&mut tx, state, reference_public_id).await?;
    let view = load_view(&mut tx, reference).await?;
    tx.commit().await?;
    Ok(view)
}

/// Refreshes one reference using its last PR `ETag` and complete observation.
///
/// A PR-level `304` never skips the bounded commits, files, or checks reads.
/// Only a completely unchanged projection advances local sync bookkeeping. A
/// new content hash appends one observation, while a duplicate hash reuses the
/// existing immutable observation. When `head_sha` changes, candidate/valid
/// evidence is made stale and every dependent coverage row becomes missing.
///
/// # Errors
///
/// Returns a scope, idempotency, connector, validation, or database error when
/// the refresh cannot be completed durably.
pub async fn refresh(
    state: &AppState,
    context: &RequestContext,
    github: &GitHubRuntime,
    reference_public_id: Uuid,
    idempotency_key: Uuid,
) -> AppResult<ExternalReferenceView> {
    let request = json!({ "external_reference_id": reference_public_id });
    let request_hash = content_hash(&request);
    let mut claim_tx = begin_scoped_transaction(state, context).await?;
    let reference = load_reference_for_update(&mut claim_tx, state, reference_public_id).await?;
    let connection_id =
        resolve_reference_connection(&mut claim_tx, &reference, github.installation_id).await?;
    let identity = GitHubPullRequestIdentity::parse(&reference.canonical_url)?;
    let previous_observation = load_latest_current_observation(
        &mut claim_tx,
        reference.id,
        reference.workspace_id,
        reference.project_id,
    )
    .await?
    .map(|record| {
        observation_from_persisted_state(&identity, &record.observed_state, record.observed_at)
    })
    .transpose()?;
    let lease = match idempotency::begin(
        &mut claim_tx,
        BeginRequest {
            workspace_id: reference.workspace_id,
            project_id: Some(reference.project_id),
            actor_id: state.actor_id,
            operation_key: REFRESH_OPERATION,
            idempotency_key: &idempotency_key.to_string(),
            request_hash: &request_hash,
        },
    )
    .await?
    {
        BeginOutcome::New { lease, .. } => lease,
        BeginOutcome::InProgress { .. } => {
            return Err(AppError::Conflict(
                "this GitHub reference refresh is already in progress; retry shortly".into(),
            ));
        }
        BeginOutcome::Replay { response, .. } => {
            claim_tx.commit().await?;
            return replay(response);
        }
    };
    let etag = reference.etag.clone();
    claim_tx.commit().await?;

    let cursor = etag
        .as_deref()
        .zip(previous_observation.as_ref())
        .map(|(etag, observation)| GitHubPullRequestCursor { etag, observation });
    let provider_result = github
        .client
        .observe_pull_request(identity.clone(), cursor)
        .await;
    match provider_result {
        Ok(GitHubObserveResult::NotModified) => {
            let mut tx = begin_scoped_transaction(state, context).await?;
            if reference.sync_status != "current" {
                append_recovery_observation(&mut tx, &reference).await?;
            }
            sqlx::query(
                "update app.external_references
                 set sync_status = 'current', last_synced_at = now(),
                     last_error_code = null, last_error_message = null
                 where id = $1 and project_id = $2 and workspace_id = $3",
            )
            .bind(reference.id)
            .bind(reference.project_id)
            .bind(reference.workspace_id)
            .execute(&mut *tx)
            .await?;
            let current = load_reference_by_internal(
                &mut tx,
                reference.id,
                reference.workspace_id,
                reference.project_id,
            )
            .await?;
            let view = load_view(&mut tx, current).await?;
            complete(&mut tx, &lease, &view).await?;
            tx.commit().await?;
            Ok(view)
        }
        Ok(GitHubObserveResult::Observed { observation, etag }) => {
            if let Err(error) = validate_observation(&identity, &observation) {
                record_provider_failure(state, context, &lease, &error).await?;
                return Err(error);
            }
            let mut tx = begin_scoped_transaction(state, context).await?;
            let connection = ConnectionRecord { id: connection_id };
            let scope = ProjectScope {
                project_id: reference.project_id,
                workspace_id: reference.workspace_id,
            };
            let view = persist_observed_reference(
                &mut tx,
                state,
                scope,
                connection,
                &identity,
                &observation,
                etag,
            )
            .await?;
            complete(&mut tx, &lease, &view).await?;
            tx.commit().await?;
            Ok(view)
        }
        Err(error) if is_unavailable(&error) => {
            let mut tx = begin_scoped_transaction(state, context).await?;
            persist_unavailable(&mut tx, &reference).await?;
            let current = load_reference_by_internal(
                &mut tx,
                reference.id,
                reference.workspace_id,
                reference.project_id,
            )
            .await?;
            let view = load_view(&mut tx, current).await?;
            complete(&mut tx, &lease, &view).await?;
            tx.commit().await?;
            Ok(view)
        }
        Err(error) => {
            let mut tx = begin_scoped_transaction(state, context).await?;
            sqlx::query(
                "update app.external_references
                 set sync_status = 'error', last_synced_at = now(),
                     last_error_code = 'github_connector_error',
                     last_error_message = 'GitHub refresh failed'
                 where id = $1 and project_id = $2 and workspace_id = $3",
            )
            .bind(reference.id)
            .bind(reference.project_id)
            .bind(reference.workspace_id)
            .execute(&mut *tx)
            .await?;
            idempotency::fail(
                &mut tx,
                &lease,
                502,
                "connector_unavailable",
                connector_failure_disposition(&error),
                json!({ "code": "connector_unavailable", "message": "GitHub refresh failed" }),
            )
            .await?;
            tx.commit().await?;
            Err(error)
        }
    }
}

/// Creates an observed-reference proof in `candidate` state only.
///
/// # Errors
///
/// Returns an error for invalid input, stale or cross-project resources,
/// idempotency conflicts, or database failures.
pub async fn create_evidence(
    state: &AppState,
    context: &RequestContext,
    reference_public_id: Uuid,
    idempotency_key: Uuid,
    input: CreateExternalEvidence,
) -> AppResult<ExternalEvidenceView> {
    validate_evidence_input(&input)?;
    let request_hash = idempotency::hash_request(&input)?;
    let mut tx = begin_scoped_transaction(state, context).await?;
    let reference = load_reference_for_update(&mut tx, state, reference_public_id).await?;
    if reference.sync_status != "current" {
        return Err(AppError::Conflict(
            "evidence requires a current external reference".into(),
        ));
    }
    let lease = match idempotency::begin(
        &mut tx,
        BeginRequest {
            workspace_id: reference.workspace_id,
            project_id: Some(reference.project_id),
            actor_id: state.actor_id,
            operation_key: EVIDENCE_CREATE_OPERATION,
            idempotency_key: &idempotency_key.to_string(),
            request_hash: &request_hash,
        },
    )
    .await?
    {
        BeginOutcome::New { lease, .. } => lease,
        BeginOutcome::InProgress { .. } => {
            return Err(AppError::Conflict(
                "this evidence creation is already in progress; retry shortly".into(),
            ));
        }
        BeginOutcome::Replay { response, .. } => {
            tx.commit().await?;
            return replay(response);
        }
    };

    let latest = load_latest_observation(
        &mut tx,
        reference.id,
        reference.workspace_id,
        reference.project_id,
    )
    .await?
    .filter(|observation| observation.observation_status == "current")
    .ok_or_else(|| AppError::Conflict("a current GitHub observation is required".into()))?;
    let head_sha = observation_head_sha(&latest.observed_state)
        .ok_or_else(|| AppError::Conflict("the current observation has no head SHA".into()))?;
    let source_reference = evidence_source_reference(&reference.canonical_url, head_sha);

    let identifiers: Option<(i64, i64, i64, i64)> = sqlx::query_as(
        "select requirement.id, version.id, deliverable.id, section.id
         from app.knowledge_entries requirement
         join app.knowledge_entry_versions version
           on version.knowledge_entry_id = requirement.id
          and version.version_number = requirement.latest_version
          and version.project_id = requirement.project_id
          and version.workspace_id = requirement.workspace_id
         join app.deliverables deliverable
           on deliverable.public_id = $3 and deliverable.project_id = requirement.project_id
          and deliverable.workspace_id = requirement.workspace_id
         join app.deliverable_sections section
           on section.public_id = $4 and section.deliverable_id = deliverable.id
          and section.project_id = deliverable.project_id
          and section.workspace_id = deliverable.workspace_id
         where requirement.public_id = $2 and requirement.project_id = $1
           and requirement.workspace_id = $5 and requirement.status = 'confirmed'
           and requirement.entry_type = 'requirement'
           and deliverable.status = 'committed'",
    )
    .bind(reference.project_id)
    .bind(input.requirement_id)
    .bind(input.deliverable_id)
    .bind(input.deliverable_section_id)
    .bind(reference.workspace_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (requirement_entry_id, requirement_version_id, deliverable_id, section_id) =
        identifiers.ok_or_else(|| {
            AppError::Invalid(
                "requirement, committed deliverable, or deliverable section is outside the reference project"
                    .into(),
            )
        })?;

    let evidence = sqlx::query_as::<_, EvidenceRecord>(
        "insert into app.evidences (
           workspace_id, project_id, requirement_entry_id, requirement_version_id,
           deliverable_id, deliverable_section_id, external_reference_id,
           evidence_type, title, description, source_reference, status
         ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'candidate')
         returning public_id,
           (select public_id from app.knowledge_entries where id = requirement_entry_id)
             as requirement_public_id,
           (select public_id from app.knowledge_entry_versions where id = requirement_version_id)
             as requirement_version_public_id,
           (select public_id from app.deliverables where id = deliverable_id)
             as deliverable_public_id,
           (select public_id from app.deliverable_sections where id = deliverable_section_id)
             as deliverable_section_public_id,
           evidence_type, title, description, source_reference, status, created_at",
    )
    .bind(reference.workspace_id)
    .bind(reference.project_id)
    .bind(requirement_entry_id)
    .bind(requirement_version_id)
    .bind(deliverable_id)
    .bind(section_id)
    .bind(reference.id)
    .bind(EVIDENCE_TYPE)
    .bind(input.title.trim())
    .bind(input.description.trim())
    .bind(source_reference)
    .fetch_one(&mut *tx)
    .await?
    .view();
    append_audit(
        &mut tx,
        reference.workspace_id,
        reference.project_id,
        state.actor_id,
        "evidence.candidate_created",
        evidence.public_id,
        None,
        json!({
            "status": "candidate",
            "external_reference_id": reference.public_id,
            "source_reference": &evidence.source_reference,
        }),
    )
    .await?;
    complete(&mut tx, &lease, &evidence).await?;
    tx.commit().await?;
    Ok(evidence)
}

/// Applies an explicit human validation or rejection to a candidate evidence.
///
/// # Errors
///
/// Returns an error for an invalid transition, an obsolete source head,
/// cross-project resources, idempotency conflicts, or database failures.
pub async fn review_evidence(
    state: &AppState,
    context: &RequestContext,
    reference_public_id: Uuid,
    evidence_public_id: Uuid,
    idempotency_key: Uuid,
    input: ReviewExternalEvidence,
) -> AppResult<ExternalEvidenceView> {
    let request_hash = idempotency::hash_request(&json!({
        "external_reference_id": reference_public_id,
        "evidence_id": evidence_public_id,
        "decision": input.decision,
    }))?;
    let mut tx = begin_scoped_transaction(state, context).await?;
    let reference = load_reference_for_update(&mut tx, state, reference_public_id).await?;
    let lease = match idempotency::begin(
        &mut tx,
        BeginRequest {
            workspace_id: reference.workspace_id,
            project_id: Some(reference.project_id),
            actor_id: state.actor_id,
            operation_key: EVIDENCE_REVIEW_OPERATION,
            idempotency_key: &idempotency_key.to_string(),
            request_hash: &request_hash,
        },
    )
    .await?
    {
        BeginOutcome::New { lease, .. } => lease,
        BeginOutcome::InProgress { .. } => {
            return Err(AppError::Conflict(
                "this evidence review is already in progress; retry shortly".into(),
            ));
        }
        BeginOutcome::Replay { response, .. } => {
            tx.commit().await?;
            return replay(response);
        }
    };

    let evidence = sqlx::query_as::<_, EvidenceIdentifiers>(
        "select id, workspace_id, project_id, requirement_entry_id,
                requirement_version_id, deliverable_id, deliverable_section_id,
                source_reference, status
         from app.evidences
         where public_id = $1 and external_reference_id = $2
           and project_id = $3 and workspace_id = $4
         for update",
    )
    .bind(evidence_public_id)
    .bind(reference.id)
    .bind(reference.project_id)
    .bind(reference.workspace_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    let target_status = decision_status(input.decision);
    if evidence.status != "candidate" && evidence.status != target_status {
        return Err(AppError::Conflict(format!(
            "evidence in state '{}' cannot transition to '{target_status}'",
            evidence.status
        )));
    }
    if target_status == "valid" {
        if reference.sync_status != "current" {
            return Err(AppError::Conflict(
                "a stale or unavailable reference cannot validate evidence".into(),
            ));
        }
        let latest = load_latest_observation(
            &mut tx,
            reference.id,
            reference.workspace_id,
            reference.project_id,
        )
        .await?
        .filter(|observation| observation.observation_status == "current")
        .ok_or_else(|| AppError::Conflict("a current observation is required".into()))?;
        let head_sha = observation_head_sha(&latest.observed_state)
            .ok_or_else(|| AppError::Conflict("the current observation has no head SHA".into()))?;
        let expected = evidence_source_reference(&reference.canonical_url, head_sha);
        if evidence.source_reference != expected {
            return Err(AppError::Conflict(
                "the candidate refers to an obsolete pull-request head".into(),
            ));
        }
    }

    sqlx::query(
        "update app.evidences set status = $2
         where id = $1 and project_id = $3 and workspace_id = $4",
    )
    .bind(evidence.id)
    .bind(target_status)
    .bind(evidence.project_id)
    .bind(evidence.workspace_id)
    .execute(&mut *tx)
    .await?;

    let deliverable_id = evidence.deliverable_id.ok_or_else(|| {
        AppError::Internal("external evidence is missing its deliverable scope".into())
    })?;
    let section_id = evidence.deliverable_section_id.ok_or_else(|| {
        AppError::Internal("external evidence is missing its section scope".into())
    })?;
    if target_status == "valid" {
        sqlx::query(
            "insert into app.requirement_coverage (
               workspace_id, project_id, requirement_entry_id, requirement_version_id,
               deliverable_id, deliverable_section_id, evidence_id, status, explanation
             ) values ($1,$2,$3,$4,$5,$6,$7,'covered',
                       'Preuve GitHub validée humainement')
             on conflict (requirement_version_id, deliverable_id, deliverable_section_id)
             do update set evidence_id = excluded.evidence_id, status = 'covered',
                           explanation = excluded.explanation, updated_at = now()",
        )
        .bind(evidence.workspace_id)
        .bind(evidence.project_id)
        .bind(evidence.requirement_entry_id)
        .bind(evidence.requirement_version_id)
        .bind(deliverable_id)
        .bind(section_id)
        .bind(evidence.id)
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query(
            "update app.requirement_coverage
             set evidence_id = null, status = 'missing',
                 explanation = 'Preuve GitHub rejetée par validation humaine',
                 updated_at = now()
             where project_id = $1 and workspace_id = $2 and evidence_id = $3",
        )
        .bind(evidence.project_id)
        .bind(evidence.workspace_id)
        .bind(evidence.id)
        .execute(&mut *tx)
        .await?;
    }
    recompute_deliverable_coverage(
        &mut tx,
        evidence.workspace_id,
        evidence.project_id,
        &[deliverable_id],
    )
    .await?;

    let reviewed = load_evidence(
        &mut tx,
        evidence.id,
        evidence.workspace_id,
        evidence.project_id,
    )
    .await?;
    append_audit(
        &mut tx,
        evidence.workspace_id,
        evidence.project_id,
        state.actor_id,
        if target_status == "valid" {
            "evidence.validated"
        } else {
            "evidence.rejected"
        },
        reviewed.public_id,
        Some(json!({ "status": evidence.status })),
        json!({ "status": target_status }),
    )
    .await?;
    complete(&mut tx, &lease, &reviewed).await?;
    tx.commit().await?;
    Ok(reviewed)
}

async fn resolve_project(
    tx: &mut Transaction<'_, Postgres>,
    state: &AppState,
    project_public_id: Uuid,
) -> AppResult<ProjectScope> {
    sqlx::query_as::<_, ProjectScope>(
        "select project.id as project_id, project.workspace_id
         from app.projects project
         join app.workspaces workspace on workspace.id = project.workspace_id
         where project.public_id = $1 and workspace.public_id = $2
           and project.status = 'active'",
    )
    .bind(project_public_id)
    .bind(state.workspace_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(AppError::NotFound)
}

async fn begin_scoped_transaction<'a>(
    state: &'a AppState,
    context: &RequestContext,
) -> AppResult<Transaction<'a, Postgres>> {
    if state.actor_id != context.actor_id
        || state.workspace_id != context.workspace_id
        || state.workspace_internal_id != context.workspace_internal_id
        || state.workspace_role != context.workspace_role
    {
        return Err(AppError::Internal(
            "request context and service scope do not match".into(),
        ));
    }
    let workspace_internal_id = context.workspace_internal_id.ok_or_else(|| {
        AppError::Internal("authenticated workspace internal id is missing".into())
    })?;
    if workspace_internal_id <= 0 {
        return Err(AppError::Internal(
            "authenticated workspace internal id is invalid".into(),
        ));
    }
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "select set_config('app.current_actor_id', $1, true),
                set_config('app.current_workspace_id', $2, true),
                set_config('app.current_workspace_role', $3, true)",
    )
    .bind(context.actor_id.to_string())
    .bind(workspace_internal_id.to_string())
    .bind(&context.workspace_role)
    .execute(&mut *tx)
    .await?;
    Ok(tx)
}

async fn resolve_connection(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: i64,
    installation_id: u64,
    requested_public_id: Option<Uuid>,
) -> AppResult<ConnectionRecord> {
    let installation_id = installation_id.to_string();
    sqlx::query_as::<_, ConnectionRecord>(
        "select id from app.tool_connections
         where workspace_id = $1 and provider = 'github' and auth_mode = 'github_app'
           and status = 'active'
           and (external_account_id = $2 or configuration ->> 'installation_id' = $2)
           and ($3::uuid is null or public_id = $3)
         order by last_verified_at desc nulls last, id desc
         limit 1",
    )
    .bind(workspace_id)
    .bind(installation_id)
    .bind(requested_public_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        AppError::Invalid(
            "no active GitHub App connection matches this workspace and installation".into(),
        )
    })
}

async fn resolve_reference_connection(
    tx: &mut Transaction<'_, Postgres>,
    reference: &ReferenceRecord,
    installation_id: u64,
) -> AppResult<i64> {
    let installation_id = installation_id.to_string();
    sqlx::query_scalar(
        "select id from app.tool_connections
         where public_id = $1 and workspace_id = $2 and provider = 'github'
           and auth_mode = 'github_app' and status = 'active'
           and (external_account_id = $3 or configuration ->> 'installation_id' = $3)",
    )
    .bind(reference.tool_connection_public_id)
    .bind(reference.workspace_id)
    .bind(installation_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| AppError::Conflict("the GitHub connection is no longer active".into()))
}

#[allow(clippy::too_many_arguments)]
async fn persist_observed_reference(
    tx: &mut Transaction<'_, Postgres>,
    state: &AppState,
    scope: ProjectScope,
    connection: ConnectionRecord,
    identity: &GitHubPullRequestIdentity,
    observation: &GitHubPullRequestObservation,
    etag: Option<String>,
) -> AppResult<ExternalReferenceView> {
    let previous: Option<PreviousReferenceState> = sqlx::query_as(
        "select last_current.observed_state ->> 'head_sha' as head_sha,
                latest.observation_status,
                latest.content_hash as observation_hash
         from app.external_references reference
         left join lateral (
           select observed_state from app.external_reference_observations
           where external_reference_id = reference.id
             and workspace_id = reference.workspace_id
             and project_id = reference.project_id
             and observation_status = 'current'
           order by observed_at desc, id desc limit 1
         ) last_current on true
         left join lateral (
           select observation_status, content_hash from app.external_reference_observations
           where external_reference_id = reference.id
             and workspace_id = reference.workspace_id
             and project_id = reference.project_id
           order by observed_at desc, id desc limit 1
         ) latest on true
         where reference.project_id = $1 and reference.workspace_id = $2
           and reference.provider = 'github' and reference.external_id = $3
         for update of reference",
    )
    .bind(scope.project_id)
    .bind(scope.workspace_id)
    .bind(identity.external_id())
    .fetch_optional(&mut **tx)
    .await?;

    let reference_id: i64 = sqlx::query_scalar(
        "insert into app.external_references (
           workspace_id, project_id, tool_connection_id, provider, object_kind,
           external_id, canonical_url, repository_full_name, display_title,
           sync_status, etag, last_synced_at, created_by_actor_id
         ) values ($1,$2,$3,'github','pull_request',$4,$5,$6,$7,
                   'current',$8,now(),$9)
         on conflict (project_id, provider, external_id)
         do update set tool_connection_id = excluded.tool_connection_id,
                       canonical_url = excluded.canonical_url,
                       repository_full_name = excluded.repository_full_name,
                       display_title = excluded.display_title,
                       sync_status = 'current',
                       etag = coalesce(excluded.etag, external_references.etag),
                       last_synced_at = now(), last_error_code = null,
                       last_error_message = null
         returning id",
    )
    .bind(scope.workspace_id)
    .bind(scope.project_id)
    .bind(connection.id)
    .bind(identity.external_id())
    .bind(identity.canonical_url())
    .bind(format!("{}/{}", identity.owner, identity.repository))
    .bind(observation.title.trim())
    .bind(etag.clone())
    .bind(state.actor_id)
    .fetch_one(&mut **tx)
    .await?;

    let mut observed_state = normalized_observation_state(observation);
    if previous
        .as_ref()
        .and_then(|state| state.observation_status.as_deref())
        == Some("unavailable")
    {
        observed_state
            .as_object_mut()
            .expect("normalized observation is always an object")
            .insert(
                "recovered_from_unavailable_hash".into(),
                Value::String(
                    previous
                        .as_ref()
                        .and_then(|state| state.observation_hash.clone())
                        .expect("an unavailable observation always has a content hash"),
                ),
            );
    }
    let hash = content_hash(&observed_state);
    sqlx::query(
        "insert into app.external_reference_observations (
           workspace_id, project_id, external_reference_id, observation_status,
           content_hash, etag, observed_state, provider_updated_at, observed_at
         ) values ($1,$2,$3,'current',$4,$5,$6,$7,$8)
         on conflict (external_reference_id, content_hash) do nothing",
    )
    .bind(scope.workspace_id)
    .bind(scope.project_id)
    .bind(reference_id)
    .bind(hash)
    .bind(etag)
    .bind(observed_state)
    .bind(observation.updated_at)
    .bind(observation.observed_at)
    .execute(&mut **tx)
    .await?;

    if head_changed(
        previous
            .as_ref()
            .and_then(|state| state.head_sha.as_deref()),
        Some(&observation.head_sha),
    ) {
        stale_dependent_evidence(
            tx,
            scope.workspace_id,
            scope.project_id,
            reference_id,
            "La pull request GitHub pointe désormais vers un nouveau head SHA",
            "stale",
        )
        .await?;
    }

    let reference =
        load_reference_by_internal(tx, reference_id, scope.workspace_id, scope.project_id).await?;
    load_view(tx, reference).await
}

async fn append_recovery_observation(
    tx: &mut Transaction<'_, Postgres>,
    reference: &ReferenceRecord,
) -> AppResult<()> {
    let previous: Option<(Value, Option<DateTime<Utc>>, String)> = sqlx::query_as(
        "select current.observed_state, current.provider_updated_at,
                unavailable.content_hash as unavailable_content_hash
         from lateral (
           select observed_state, provider_updated_at
           from app.external_reference_observations
           where external_reference_id = $1 and workspace_id = $2 and project_id = $3
             and observation_status = 'current'
           order by observed_at desc, id desc limit 1
         ) current
         join lateral (
           select content_hash
           from app.external_reference_observations
           where external_reference_id = $1 and workspace_id = $2 and project_id = $3
             and observation_status = 'unavailable'
           order by observed_at desc, id desc limit 1
         ) unavailable on true",
    )
    .bind(reference.id)
    .bind(reference.workspace_id)
    .bind(reference.project_id)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((mut observed_state, provider_updated_at, unavailable_hash)) = previous else {
        return Ok(());
    };
    observed_state
        .as_object_mut()
        .ok_or_else(|| AppError::Internal("stored GitHub observation is not an object".into()))?
        .insert(
            "recovered_from_unavailable_hash".into(),
            Value::String(unavailable_hash),
        );
    let hash = content_hash(&observed_state);
    sqlx::query(
        "insert into app.external_reference_observations (
           workspace_id, project_id, external_reference_id, observation_status,
           content_hash, etag, observed_state, provider_updated_at, observed_at
         ) values ($1,$2,$3,'current',$4,$5,$6,$7,now())
         on conflict (external_reference_id, content_hash) do nothing",
    )
    .bind(reference.workspace_id)
    .bind(reference.project_id)
    .bind(reference.id)
    .bind(hash)
    .bind(reference.etag.clone())
    .bind(observed_state)
    .bind(provider_updated_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn persist_unavailable(
    tx: &mut Transaction<'_, Postgres>,
    reference: &ReferenceRecord,
) -> AppResult<()> {
    let last_current_hash: Option<String> = sqlx::query_scalar(
        "select content_hash from app.external_reference_observations
         where external_reference_id = $1 and workspace_id = $2 and project_id = $3
           and observation_status = 'current'
         order by observed_at desc, id desc limit 1",
    )
    .bind(reference.id)
    .bind(reference.workspace_id)
    .bind(reference.project_id)
    .fetch_optional(&mut **tx)
    .await?;
    let state = json!({
        "provider": "github",
        "availability": "unavailable",
        "reason_code": "not_found_or_forbidden",
        "last_current_content_hash": last_current_hash,
    });
    sqlx::query(
        "insert into app.external_reference_observations (
           workspace_id, project_id, external_reference_id, observation_status,
           content_hash, etag, observed_state, observed_at
         ) values ($1,$2,$3,'unavailable',$4,$5,$6,now())
         on conflict (external_reference_id, content_hash) do nothing",
    )
    .bind(reference.workspace_id)
    .bind(reference.project_id)
    .bind(reference.id)
    .bind(content_hash(&state))
    .bind(reference.etag.clone())
    .bind(state)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "update app.external_references
         set sync_status = 'unavailable', last_synced_at = now(),
             last_error_code = 'github_unavailable',
             last_error_message = 'GitHub reference is unavailable'
         where id = $1 and project_id = $2 and workspace_id = $3",
    )
    .bind(reference.id)
    .bind(reference.project_id)
    .bind(reference.workspace_id)
    .execute(&mut **tx)
    .await?;
    stale_dependent_evidence(
        tx,
        reference.workspace_id,
        reference.project_id,
        reference.id,
        "La référence GitHub est devenue indisponible",
        "unavailable",
    )
    .await
}

async fn stale_dependent_evidence(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: i64,
    project_id: i64,
    reference_id: i64,
    explanation: &str,
    evidence_status: &str,
) -> AppResult<()> {
    let evidence_ids: Vec<i64> = sqlx::query_scalar(
        "update app.evidences
         set status = $4
         where workspace_id = $1 and project_id = $2 and external_reference_id = $3
           and status in ('candidate','valid')
         returning id",
    )
    .bind(workspace_id)
    .bind(project_id)
    .bind(reference_id)
    .bind(evidence_status)
    .fetch_all(&mut **tx)
    .await?;
    if evidence_ids.is_empty() {
        return Ok(());
    }
    let deliverable_ids: Vec<i64> = sqlx::query_scalar(
        "update app.requirement_coverage
         set status = 'missing', explanation = $3, updated_at = now()
         where workspace_id = $1 and project_id = $2 and evidence_id = any($4)
         returning deliverable_id",
    )
    .bind(workspace_id)
    .bind(project_id)
    .bind(explanation)
    .bind(&evidence_ids)
    .fetch_all(&mut **tx)
    .await?;
    recompute_deliverable_coverage(tx, workspace_id, project_id, &deliverable_ids).await
}

async fn recompute_deliverable_coverage(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: i64,
    project_id: i64,
    deliverable_ids: &[i64],
) -> AppResult<()> {
    if deliverable_ids.is_empty() {
        return Ok(());
    }
    sqlx::query(
        "update app.deliverables deliverable
         set coverage_status = case
           when not exists (
             select 1 from app.requirement_coverage coverage
             where coverage.deliverable_id = deliverable.id
               and coverage.workspace_id = $1 and coverage.project_id = $2
           ) then 'missing'
           when not exists (
             select 1 from app.requirement_coverage coverage
             where coverage.deliverable_id = deliverable.id
               and coverage.workspace_id = $1 and coverage.project_id = $2
               and coverage.status <> 'covered'
           ) then 'covered'
           when exists (
             select 1 from app.requirement_coverage coverage
             where coverage.deliverable_id = deliverable.id
               and coverage.workspace_id = $1 and coverage.project_id = $2
               and coverage.status = 'covered'
           ) then 'partial'
           else 'missing'
         end,
         updated_at = now()
         where deliverable.workspace_id = $1 and deliverable.project_id = $2
           and deliverable.id = any($3)",
    )
    .bind(workspace_id)
    .bind(project_id)
    .bind(deliverable_ids)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn load_reference(
    tx: &mut Transaction<'_, Postgres>,
    state: &AppState,
    reference_public_id: Uuid,
) -> AppResult<ReferenceRecord> {
    sqlx::query_as::<_, ReferenceRecord>(REFERENCE_SELECT_BY_PUBLIC_ID)
        .bind(state.workspace_id)
        .bind(reference_public_id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(AppError::NotFound)
}

async fn load_reference_for_update(
    tx: &mut Transaction<'_, Postgres>,
    state: &AppState,
    reference_public_id: Uuid,
) -> AppResult<ReferenceRecord> {
    sqlx::query_as::<_, ReferenceRecord>(REFERENCE_SELECT_BY_PUBLIC_ID_FOR_UPDATE)
        .bind(state.workspace_id)
        .bind(reference_public_id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(AppError::NotFound)
}

async fn load_reference_by_internal(
    tx: &mut Transaction<'_, Postgres>,
    reference_id: i64,
    workspace_id: i64,
    project_id: i64,
) -> AppResult<ReferenceRecord> {
    sqlx::query_as::<_, ReferenceRecord>(REFERENCE_SELECT_BY_INTERNAL_ID)
        .bind(reference_id)
        .bind(workspace_id)
        .bind(project_id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(AppError::NotFound)
}

async fn load_latest_observation(
    tx: &mut Transaction<'_, Postgres>,
    reference_id: i64,
    workspace_id: i64,
    project_id: i64,
) -> AppResult<Option<ObservationRecord>> {
    Ok(sqlx::query_as::<_, ObservationRecord>(
        "select public_id, observation_status, content_hash, etag, observed_state,
                provider_updated_at, observed_at
         from app.external_reference_observations
         where external_reference_id = $1 and workspace_id = $2 and project_id = $3
         order by observed_at desc, id desc limit 1",
    )
    .bind(reference_id)
    .bind(workspace_id)
    .bind(project_id)
    .fetch_optional(&mut **tx)
    .await?)
}

async fn load_latest_current_observation(
    tx: &mut Transaction<'_, Postgres>,
    reference_id: i64,
    workspace_id: i64,
    project_id: i64,
) -> AppResult<Option<ObservationRecord>> {
    Ok(sqlx::query_as::<_, ObservationRecord>(
        "select public_id, observation_status, content_hash, etag, observed_state,
                provider_updated_at, observed_at
         from app.external_reference_observations
         where external_reference_id = $1 and workspace_id = $2 and project_id = $3
           and observation_status = 'current'
         order by observed_at desc, id desc limit 1",
    )
    .bind(reference_id)
    .bind(workspace_id)
    .bind(project_id)
    .fetch_optional(&mut **tx)
    .await?)
}

async fn load_view(
    tx: &mut Transaction<'_, Postgres>,
    reference: ReferenceRecord,
) -> AppResult<ExternalReferenceView> {
    let latest_observation = load_latest_observation(
        tx,
        reference.id,
        reference.workspace_id,
        reference.project_id,
    )
    .await?
    .map(ObservationRecord::view);
    let evidences = sqlx::query_as::<_, EvidenceRecord>(EVIDENCE_SELECT)
        .bind(reference.id)
        .bind(reference.project_id)
        .bind(reference.workspace_id)
        .fetch_all(&mut **tx)
        .await?
        .into_iter()
        .map(EvidenceRecord::view)
        .collect();
    Ok(ExternalReferenceView {
        reference: reference.summary(),
        latest_observation,
        evidences,
    })
}

async fn load_evidence(
    tx: &mut Transaction<'_, Postgres>,
    evidence_id: i64,
    workspace_id: i64,
    project_id: i64,
) -> AppResult<ExternalEvidenceView> {
    sqlx::query_as::<_, EvidenceRecord>(
        "select evidence.public_id,
                requirement.public_id as requirement_public_id,
                version.public_id as requirement_version_public_id,
                deliverable.public_id as deliverable_public_id,
                section.public_id as deliverable_section_public_id,
                evidence.evidence_type, evidence.title, evidence.description,
                evidence.source_reference, evidence.status, evidence.created_at
         from app.evidences evidence
         join app.knowledge_entries requirement
           on requirement.id = evidence.requirement_entry_id
          and requirement.workspace_id = evidence.workspace_id
          and requirement.project_id = evidence.project_id
         join app.knowledge_entry_versions version
           on version.id = evidence.requirement_version_id
          and version.workspace_id = evidence.workspace_id
          and version.project_id = evidence.project_id
         join app.deliverables deliverable
           on deliverable.id = evidence.deliverable_id
          and deliverable.workspace_id = evidence.workspace_id
          and deliverable.project_id = evidence.project_id
         join app.deliverable_sections section
           on section.id = evidence.deliverable_section_id
          and section.workspace_id = evidence.workspace_id
          and section.project_id = evidence.project_id
         where evidence.id = $1 and evidence.workspace_id = $2 and evidence.project_id = $3",
    )
    .bind(evidence_id)
    .bind(workspace_id)
    .bind(project_id)
    .fetch_optional(&mut **tx)
    .await?
    .map(EvidenceRecord::view)
    .ok_or(AppError::NotFound)
}

async fn complete<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    lease: &IdempotencyLease,
    value: &T,
) -> AppResult<()> {
    let body = serde_json::to_value(value)
        .map_err(|error| AppError::Internal(format!("failed to store response: {error}")))?;
    idempotency::complete(tx, lease, 200, body).await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn append_audit(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: i64,
    project_id: i64,
    actor_id: Uuid,
    action: &str,
    object_public_id: Uuid,
    before_state: Option<Value>,
    after_state: Value,
) -> AppResult<()> {
    sqlx::query(
        "insert into app.audit_events (
           workspace_id, project_id, actor_id, action, object_kind,
           object_public_id, before_state, after_state
         ) values ($1,$2,$3,$4,'evidence',$5,$6,$7)",
    )
    .bind(workspace_id)
    .bind(project_id)
    .bind(actor_id)
    .bind(action)
    .bind(object_public_id)
    .bind(before_state)
    .bind(after_state)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn record_provider_failure(
    state: &AppState,
    context: &RequestContext,
    lease: &IdempotencyLease,
    error: &AppError,
) -> AppResult<()> {
    let mut tx = begin_scoped_transaction(state, context).await?;
    idempotency::fail(
        &mut tx,
        lease,
        502,
        "connector_unavailable",
        connector_failure_disposition(error),
        json!({ "code": "connector_unavailable", "message": "GitHub import failed" }),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

fn connector_failure_disposition(error: &AppError) -> FailureDisposition {
    let AppError::Connector(message) = error else {
        return FailureDisposition::Permanent;
    };
    let status = message
        .split_whitespace()
        .last()
        .and_then(|value| value.parse::<u16>().ok());
    if message.contains("request failed")
        || message.contains("exhausted retries")
        || status.is_some_and(|status| status == 429 || status >= 500)
    {
        FailureDisposition::Retryable
    } else {
        FailureDisposition::Permanent
    }
}

fn replay<T: for<'de> Deserialize<'de>>(response: idempotency::StoredResponse) -> AppResult<T> {
    if response.failed {
        return Err(AppError::Connector(
            "the previous GitHub operation failed; use a new idempotency key to retry".into(),
        ));
    }
    serde_json::from_value(response.body)
        .map_err(|_| AppError::Internal("stored idempotent response is invalid".into()))
}

fn validate_evidence_input(input: &CreateExternalEvidence) -> AppResult<()> {
    if input.title.trim().is_empty() {
        return Err(AppError::Invalid("evidence title is required".into()));
    }
    if input.title.chars().count() > 240 {
        return Err(AppError::Invalid(
            "evidence title must be at most 240 characters".into(),
        ));
    }
    if input.description.chars().count() > 4_000 {
        return Err(AppError::Invalid(
            "evidence description must be at most 4000 characters".into(),
        ));
    }
    Ok(())
}

fn validate_observation(
    expected: &GitHubPullRequestIdentity,
    observation: &GitHubPullRequestObservation,
) -> AppResult<()> {
    if &observation.identity != expected
        || observation.canonical_url != expected.canonical_url()
        || observation.title.trim().is_empty()
        || !matches!(observation.state.as_str(), "open" | "closed")
        || !valid_git_object_id(&observation.base_sha)
        || !valid_git_object_id(&observation.head_sha)
        || observation
            .commits
            .iter()
            .any(|sha| !valid_git_object_id(sha))
    {
        return Err(AppError::Connector(
            "GitHub returned an invalid pull-request projection".into(),
        ));
    }
    Ok(())
}

fn valid_git_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn decision_status(decision: EvidenceDecision) -> &'static str {
    match decision {
        EvidenceDecision::Validate => "valid",
        EvidenceDecision::Reject => "rejected",
    }
}

fn is_unavailable(error: &AppError) -> bool {
    match error {
        AppError::Connector(message) => {
            message.contains("status 403") || message.contains("status 404")
        }
        _ => false,
    }
}

fn evidence_source_reference(canonical_url: &str, head_sha: &str) -> String {
    format!("{canonical_url}@{head_sha}")
}

fn observation_head_sha(state: &Value) -> Option<&str> {
    state.get("head_sha").and_then(Value::as_str)
}

fn head_changed(previous: Option<&str>, current: Option<&str>) -> bool {
    matches!((previous, current), (Some(previous), Some(current)) if previous != current)
}

fn normalized_observation_state(observation: &GitHubPullRequestObservation) -> Value {
    let mut changed_paths = observation.changed_paths.clone();
    changed_paths.sort();
    changed_paths.dedup();
    let mut checks = observation.checks.clone();
    checks.sort_by(|left, right| check_key(left).cmp(&check_key(right)));
    json!({
        "provider": "github",
        "object_kind": "pull_request",
        "canonical_url": observation.canonical_url,
        "external_id": observation.identity.external_id(),
        "title": observation.title,
        "state": observation.state,
        "draft": observation.draft,
        "merged": observation.merged,
        "base_sha": observation.base_sha,
        "head_sha": observation.head_sha,
        "commits": observation.commits,
        "changed_paths": changed_paths,
        "checks": checks,
        "provider_created_at": observation.created_at,
        "provider_updated_at": observation.updated_at,
    })
}

#[derive(Debug, Deserialize)]
struct PersistedGitHubPullRequestState {
    provider: String,
    object_kind: String,
    canonical_url: String,
    external_id: String,
    title: String,
    state: String,
    draft: bool,
    merged: bool,
    base_sha: String,
    head_sha: String,
    #[serde(default)]
    commits: Vec<String>,
    #[serde(default)]
    changed_paths: Vec<String>,
    #[serde(default)]
    checks: Vec<GitHubCheckObservation>,
    provider_created_at: Option<DateTime<Utc>>,
    provider_updated_at: Option<DateTime<Utc>>,
}

fn observation_from_persisted_state(
    identity: &GitHubPullRequestIdentity,
    observed_state: &Value,
    observed_at: DateTime<Utc>,
) -> AppResult<GitHubPullRequestObservation> {
    let persisted: PersistedGitHubPullRequestState = serde_json::from_value(observed_state.clone())
        .map_err(|_| {
            AppError::Internal("stored GitHub observation has an invalid projection".into())
        })?;
    if persisted.provider != "github"
        || persisted.object_kind != "pull_request"
        || persisted.canonical_url != identity.canonical_url()
        || persisted.external_id != identity.external_id()
    {
        return Err(AppError::Internal(
            "stored GitHub observation does not match its reference".into(),
        ));
    }
    let observation = GitHubPullRequestObservation {
        identity: identity.clone(),
        canonical_url: persisted.canonical_url,
        title: persisted.title,
        state: persisted.state,
        draft: persisted.draft,
        merged: persisted.merged,
        base_sha: persisted.base_sha,
        head_sha: persisted.head_sha,
        commits: persisted.commits,
        changed_paths: persisted.changed_paths,
        checks: persisted.checks,
        created_at: persisted.provider_created_at,
        updated_at: persisted.provider_updated_at,
        observed_at,
    };
    validate_observation(identity, &observation).map_err(|_| {
        AppError::Internal("stored GitHub observation cannot be used for refresh".into())
    })?;
    Ok(observation)
}

fn check_key(check: &GitHubCheckObservation) -> (&str, &str, Option<&str>, Option<&str>) {
    (
        check.name.as_str(),
        check.status.as_str(),
        check.conclusion.as_deref(),
        check.details_url.as_deref(),
    )
}

fn content_hash(value: &Value) -> String {
    let canonical = canonicalize_json(value);
    format!("{:x}", Sha256::digest(canonical.to_string().as_bytes()))
}

fn canonicalize_json(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut keys: Vec<_> = object.keys().collect();
            keys.sort();
            let mut canonical = serde_json::Map::new();
            for key in keys {
                canonical.insert(key.clone(), canonicalize_json(&object[key]));
            }
            Value::Object(canonical)
        }
        Value::Array(values) => Value::Array(values.iter().map(canonicalize_json).collect()),
        scalar => scalar.clone(),
    }
}

const REFERENCE_SELECT_BY_PROJECT: &str =
    "select reference.id, reference.workspace_id, reference.project_id,
            reference.public_id, project.public_id as project_public_id,
            connection.public_id as tool_connection_public_id,
            reference.provider, reference.object_kind, reference.external_id,
            reference.canonical_url, reference.repository_full_name,
            reference.display_title, reference.sync_status, reference.etag,
            reference.last_synced_at, reference.last_error_code,
            reference.created_at, reference.updated_at
     from app.external_references reference
     join app.projects project on project.id = reference.project_id
       and project.workspace_id = reference.workspace_id
     join app.workspaces workspace on workspace.id = reference.workspace_id
     join app.tool_connections connection on connection.id = reference.tool_connection_id
       and connection.workspace_id = reference.workspace_id
     where workspace.public_id = $1 and project.public_id = $2
     order by reference.updated_at desc, reference.id desc";

const REFERENCE_SELECT_BY_PUBLIC_ID: &str =
    "select reference.id, reference.workspace_id, reference.project_id,
            reference.public_id, project.public_id as project_public_id,
            connection.public_id as tool_connection_public_id,
            reference.provider, reference.object_kind, reference.external_id,
            reference.canonical_url, reference.repository_full_name,
            reference.display_title, reference.sync_status, reference.etag,
            reference.last_synced_at, reference.last_error_code,
            reference.created_at, reference.updated_at
     from app.external_references reference
     join app.projects project on project.id = reference.project_id
       and project.workspace_id = reference.workspace_id
     join app.workspaces workspace on workspace.id = reference.workspace_id
     join app.tool_connections connection on connection.id = reference.tool_connection_id
       and connection.workspace_id = reference.workspace_id
     where workspace.public_id = $1 and reference.public_id = $2";

const REFERENCE_SELECT_BY_PUBLIC_ID_FOR_UPDATE: &str =
    "select reference.id, reference.workspace_id, reference.project_id,
            reference.public_id, project.public_id as project_public_id,
            connection.public_id as tool_connection_public_id,
            reference.provider, reference.object_kind, reference.external_id,
            reference.canonical_url, reference.repository_full_name,
            reference.display_title, reference.sync_status, reference.etag,
            reference.last_synced_at, reference.last_error_code,
            reference.created_at, reference.updated_at
     from app.external_references reference
     join app.projects project on project.id = reference.project_id
       and project.workspace_id = reference.workspace_id
     join app.workspaces workspace on workspace.id = reference.workspace_id
     join app.tool_connections connection on connection.id = reference.tool_connection_id
       and connection.workspace_id = reference.workspace_id
     where workspace.public_id = $1 and reference.public_id = $2
     for update of reference";

const REFERENCE_SELECT_BY_INTERNAL_ID: &str =
    "select reference.id, reference.workspace_id, reference.project_id,
            reference.public_id, project.public_id as project_public_id,
            connection.public_id as tool_connection_public_id,
            reference.provider, reference.object_kind, reference.external_id,
            reference.canonical_url, reference.repository_full_name,
            reference.display_title, reference.sync_status, reference.etag,
            reference.last_synced_at, reference.last_error_code,
            reference.created_at, reference.updated_at
     from app.external_references reference
     join app.projects project on project.id = reference.project_id
       and project.workspace_id = reference.workspace_id
     join app.tool_connections connection on connection.id = reference.tool_connection_id
       and connection.workspace_id = reference.workspace_id
     where reference.id = $1 and reference.workspace_id = $2
       and reference.project_id = $3";

const EVIDENCE_SELECT: &str = "select evidence.public_id,
            requirement.public_id as requirement_public_id,
            version.public_id as requirement_version_public_id,
            deliverable.public_id as deliverable_public_id,
            section.public_id as deliverable_section_public_id,
            evidence.evidence_type, evidence.title, evidence.description,
            evidence.source_reference, evidence.status, evidence.created_at
     from app.evidences evidence
     join app.knowledge_entries requirement
       on requirement.id = evidence.requirement_entry_id
      and requirement.workspace_id = evidence.workspace_id
      and requirement.project_id = evidence.project_id
     join app.knowledge_entry_versions version
       on version.id = evidence.requirement_version_id
      and version.workspace_id = evidence.workspace_id
      and version.project_id = evidence.project_id
     join app.deliverables deliverable
       on deliverable.id = evidence.deliverable_id
      and deliverable.workspace_id = evidence.workspace_id
      and deliverable.project_id = evidence.project_id
     join app.deliverable_sections section
       on section.id = evidence.deliverable_section_id
      and section.workspace_id = evidence.workspace_id
      and section.project_id = evidence.project_id
     where evidence.external_reference_id = $1 and evidence.project_id = $2
       and evidence.workspace_id = $3
     order by evidence.created_at desc, evidence.id desc";

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(observed_at: &str, paths: Vec<&str>) -> GitHubPullRequestObservation {
        GitHubPullRequestObservation {
            identity: GitHubPullRequestIdentity {
                owner: "acme".into(),
                repository: "context".into(),
                number: 7,
            },
            canonical_url: "https://github.com/acme/context/pull/7".into(),
            title: "Keep product context explicit".into(),
            state: "open".into(),
            draft: false,
            merged: false,
            base_sha: "a".repeat(40),
            head_sha: "b".repeat(40),
            commits: vec!["b".repeat(40)],
            changed_paths: paths.into_iter().map(str::to_owned).collect(),
            checks: vec![GitHubCheckObservation {
                name: "test".into(),
                status: "completed".into(),
                conclusion: Some("success".into()),
                details_url: Some("https://github.com/acme/context/actions/runs/1".into()),
            }],
            created_at: None,
            updated_at: None,
            observed_at: observed_at.parse().expect("valid timestamp"),
        }
    }

    #[test]
    fn observation_hash_ignores_local_time_and_normalizes_path_order() {
        let first = normalized_observation_state(&observation(
            "2026-08-24T10:00:00Z",
            vec!["src/b.rs", "src/a.rs"],
        ));
        let second = normalized_observation_state(&observation(
            "2026-08-25T10:00:00Z",
            vec!["src/a.rs", "src/b.rs", "src/a.rs"],
        ));
        assert_eq!(first, second);
        assert_eq!(content_hash(&first), content_hash(&second));
        assert_eq!(content_hash(&first).len(), 64);
    }

    #[test]
    fn persisted_current_observation_reconstructs_the_conditional_projection() {
        let original = observation(
            "2026-08-24T10:00:00Z",
            vec!["src/b.rs", "src/a.rs", "src/a.rs"],
        );
        let persisted = normalized_observation_state(&original);
        let restored = observation_from_persisted_state(
            &original.identity,
            &persisted,
            "2026-08-25T10:00:00Z"
                .parse()
                .expect("valid restore timestamp"),
        )
        .expect("a current observation should reconstruct");

        assert_eq!(
            normalized_observation_state(&restored),
            normalized_observation_state(&original)
        );
        assert_ne!(restored.observed_at, original.observed_at);
    }

    #[test]
    fn head_change_requires_two_known_different_shas() {
        assert!(head_changed(Some("old"), Some("new")));
        assert!(!head_changed(Some("same"), Some("same")));
        assert!(!head_changed(None, Some("first")));
        assert!(!head_changed(Some("old"), None));
    }

    #[test]
    fn evidence_validation_is_an_explicit_non_candidate_decision() {
        assert_eq!(decision_status(EvidenceDecision::Validate), "valid");
        assert_eq!(decision_status(EvidenceDecision::Reject), "rejected");
        let serialized =
            serde_json::to_string(&EvidenceDecision::Validate).expect("decision should serialize");
        assert_eq!(serialized, "\"validate\"");
    }

    #[test]
    fn evidence_reference_is_bound_to_the_exact_pull_request_head() {
        assert_eq!(
            evidence_source_reference("https://github.com/acme/context/pull/7", "abc123"),
            "https://github.com/acme/context/pull/7@abc123"
        );
    }

    #[test]
    fn hash_is_independent_of_object_key_order() {
        let left = json!({ "b": 2, "a": { "d": 4, "c": 3 } });
        let right: Value =
            serde_json::from_str(r#"{"a":{"c":3,"d":4},"b":2}"#).expect("valid JSON");
        assert_eq!(content_hash(&left), content_hash(&right));
    }

    #[test]
    fn provider_projection_requires_canonical_identity_and_full_git_ids() {
        let valid = observation("2026-08-24T10:00:00Z", vec!["src/lib.rs"]);
        assert!(validate_observation(&valid.identity, &valid).is_ok());

        let mut invalid = valid.clone();
        invalid.head_sha = "not-a-full-sha".into();
        assert!(validate_observation(&valid.identity, &invalid).is_err());
        assert!(!valid_git_object_id("abc123"));
        assert!(valid_git_object_id(&"f".repeat(40)));
    }

    #[test]
    fn only_not_found_or_forbidden_are_persisted_as_unavailable() {
        assert!(is_unavailable(&AppError::Connector(
            "GitHub request returned status 404".into()
        )));
        assert!(is_unavailable(&AppError::Connector(
            "GitHub request returned status 403".into()
        )));
        assert!(!is_unavailable(&AppError::Connector(
            "GitHub request returned status 500".into()
        )));
    }

    #[test]
    fn connector_retry_policy_separates_transport_and_permanent_failures() {
        for message in [
            "GitHub request failed",
            "GitHub request exhausted retries",
            "GitHub request returned status 429",
            "GitHub request returned status 503",
        ] {
            assert_eq!(
                connector_failure_disposition(&AppError::Connector(message.into())),
                FailureDisposition::Retryable
            );
        }
        for message in [
            "GitHub App private key is invalid",
            "GitHub request returned status 401",
            "GitHub returned invalid JSON",
            "GitHub returned an invalid pull-request projection",
        ] {
            assert_eq!(
                connector_failure_disposition(&AppError::Connector(message.into())),
                FailureDisposition::Permanent
            );
        }
    }
}
