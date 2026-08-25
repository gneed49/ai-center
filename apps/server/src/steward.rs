//! Generic, source-bound Steward analysis.
//!
//! The module deliberately separates short database transactions from the
//! provider call. A first transaction captures a versioned project snapshot
//! and creates a durable `model_run`; the model is then called without holding
//! a database connection; a second transaction rejects stale graph results and
//! persists append-only assessments and their exact sources.

#![allow(clippy::missing_errors_doc, clippy::too_many_lines)]

use std::{
    cmp::Reverse,
    collections::{BTreeSet, HashMap, HashSet},
    time::Duration,
};

use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, Postgres, Transaction};
use tokio::{sync::mpsc, task::JoinHandle};
use uuid::Uuid;

use crate::{
    agent::{
        AgentRunMetadata, ContradictionAssessmentDraft, EngineOutput, StewardInput, StewardOutput,
    },
    error::{AppError, AppResult},
    outbox::{ClaimedDomainEvent, Outbox, OutboxError, OutboxPolicy},
    service::AppState,
};

const STEWARD_PROMPT_VERSION: &str = "alpha-steward-v1";
const STEWARD_SCHEMA_VERSION: &str = "alpha-steward-v1";
const MAX_CONFIGURED_VERSIONS: u32 = 512;
const MAX_CONFIGURED_PAIRS: usize = 128;
const MAX_TITLE_CHARS: usize = 500;
const MAX_EXPLANATION_CHARS: usize = 4_000;
const STEWARD_OUTBOX_BATCH_SIZE: u32 = 64;
const STEWARD_TRIGGER_CAPACITY: usize = 64;
const DEFAULT_STEWARD_SCAN_INTERVAL: Duration = Duration::from_secs(15);
const DEFAULT_STEWARD_SCAN_WORKSPACE_LIMIT: u32 = 16;
const MIN_STEWARD_SCAN_INTERVAL: Duration = Duration::from_secs(1);
const MAX_STEWARD_SCAN_INTERVAL: Duration = Duration::from_secs(5 * 60);
const MAX_STEWARD_SCAN_WORKSPACE_LIMIT: u32 = 128;
const STEWARD_EVENT_TYPES: [&str; 2] = ["knowledge.committed", "knowledge.revised"];

/// Bounded inputs for one Steward run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StewardConfig {
    pub max_versions: u32,
    pub max_candidate_pairs: usize,
}

impl Default for StewardConfig {
    fn default() -> Self {
        Self {
            max_versions: 128,
            max_candidate_pairs: 48,
        }
    }
}

impl StewardConfig {
    fn validate(self) -> AppResult<Self> {
        if !(2..=MAX_CONFIGURED_VERSIONS).contains(&self.max_versions) {
            return Err(AppError::Invalid(format!(
                "max_versions must be between 2 and {MAX_CONFIGURED_VERSIONS}"
            )));
        }
        if !(1..=MAX_CONFIGURED_PAIRS).contains(&self.max_candidate_pairs) {
            return Err(AppError::Invalid(format!(
                "max_candidate_pairs must be between 1 and {MAX_CONFIGURED_PAIRS}"
            )));
        }
        Ok(self)
    }
}

/// Bounds the periodic recovery scan. The production default performs one
/// private discovery query every 15 seconds and returns at most 16 workspace
/// scopes. A scan never reads event payloads or processes outside those scopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StewardSupervisorPolicy {
    pub scan_interval: Duration,
    pub max_workspaces_per_scan: u32,
}

impl Default for StewardSupervisorPolicy {
    fn default() -> Self {
        Self {
            scan_interval: DEFAULT_STEWARD_SCAN_INTERVAL,
            max_workspaces_per_scan: DEFAULT_STEWARD_SCAN_WORKSPACE_LIMIT,
        }
    }
}

impl StewardSupervisorPolicy {
    fn validate(self) -> Result<Self, OutboxError> {
        if !(MIN_STEWARD_SCAN_INTERVAL..=MAX_STEWARD_SCAN_INTERVAL).contains(&self.scan_interval) {
            return Err(OutboxError::InvalidPolicy(
                "Steward scan interval must be between 1 second and 5 minutes",
            ));
        }
        if !(1..=MAX_STEWARD_SCAN_WORKSPACE_LIMIT).contains(&self.max_workspaces_per_scan) {
            return Err(OutboxError::InvalidPolicy(
                "Steward scan workspace limit must be between 1 and 128",
            ));
        }
        Ok(self)
    }
}

/// Durable result returned after the provider output has been validated and
/// the graph version has been checked again.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StewardRunResult {
    pub project_public_id: Uuid,
    pub source_graph_version: i64,
    pub model_run_public_id: Option<Uuid>,
    pub candidate_pair_count: usize,
    pub assessment_count: usize,
    pub contradiction_count: usize,
    pub insight_public_ids: Vec<Uuid>,
}

/// Result of consuming an already-leased domain event.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum StewardEventOutcome {
    Ignored,
    Assessed { result: StewardRunResult },
}

/// Aggregate proof that one request-scoped drain followed the complete outbox
/// lifecycle. Provider/model failures are counted after `mark_failed` has
/// scheduled their retry or dead-lettered them; they do not abort later events
/// in the same claimed batch.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct StewardDrainSummary {
    pub claimed: usize,
    pub processed: usize,
    pub coalesced: usize,
    pub failed: usize,
    pub reclaimed_for_retry: i64,
    pub reclaimed_to_dead_letter: i64,
    pub insight_public_ids: Vec<Uuid>,
}

/// Non-blocking post-commit trigger held by request-scoped [`AppState`] clones.
///
/// The scoped state itself is queued, so every claim, read and transition uses
/// the authenticated actor/workspace GUCs. The supervisor never owns a global
/// or RLS-bypassing database state.
#[derive(Clone)]
pub struct StewardDrainTrigger {
    sender: mpsc::Sender<AppState>,
}

/// Owns and observes the background drain loop used in `OpenAI` mode.
///
/// Dropping all [`StewardDrainTrigger`] values closes the channel. `shutdown`
/// then waits for the currently supervised drain and exits naturally.
pub struct StewardDrainSupervisor {
    task: JoinHandle<()>,
}

impl StewardDrainTrigger {
    pub fn trigger_after_commit(&self, state: &AppState) -> AppResult<()> {
        ensure_mutating_role(state)?;
        request_workspace_id(state)?;
        self.sender.try_send(state.clone()).map_err(|error| {
            AppError::Internal(format!("Steward outbox trigger was not accepted: {error}"))
        })
    }
}

impl StewardDrainSupervisor {
    /// Starts the `OpenAI` outbox supervisor on the current Tokio runtime.
    ///
    /// `scanner_state` is deliberately unscoped. It is used only to call the
    /// private, allowlisted workspace discovery function. Each returned row is
    /// converted into a scoped state before any event or project is read.
    pub fn start(scanner_state: AppState) -> Result<(StewardDrainTrigger, Self), OutboxError> {
        Self::start_with_policy(scanner_state, StewardSupervisorPolicy::default())
    }

    /// Starts the supervisor with an explicit bounded scan policy. This is
    /// public so integration tests can prove restart recovery without changing
    /// the production polling interval.
    pub fn start_with_policy(
        scanner_state: AppState,
        policy: StewardSupervisorPolicy,
    ) -> Result<(StewardDrainTrigger, Self), OutboxError> {
        Self::start_with_policies(scanner_state, policy, OutboxPolicy::default())
    }

    /// Starts with explicit supervisor and outbox policies. Production uses
    /// [`Self::start`]; this hook lets database integration tests exercise an
    /// expired lease without waiting for the production backoff.
    pub fn start_with_policies(
        mut scanner_state: AppState,
        supervisor_policy: StewardSupervisorPolicy,
        outbox_policy: OutboxPolicy,
    ) -> Result<(StewardDrainTrigger, Self), OutboxError> {
        let supervisor_policy = supervisor_policy.validate()?;
        let outbox = Outbox::new(outbox_policy)?;
        let (sender, receiver) = mpsc::channel(STEWARD_TRIGGER_CAPACITY);
        // Never let the scanner template retain a sender to its own channel.
        // Channel closure therefore remains a natural and bounded shutdown.
        scanner_state.steward_trigger = None;
        let task = tokio::spawn(supervise_drains(
            receiver,
            outbox,
            scanner_state,
            supervisor_policy,
        ));
        Ok((StewardDrainTrigger { sender }, Self { task }))
    }

    /// Waits for channel closure and the active drain. No infinite polling task
    /// remains after the HTTP server has released all trigger handles.
    pub async fn shutdown(self) {
        if let Err(error) = self.task.await {
            tracing::error!(%error, "Steward outbox supervisor stopped unexpectedly");
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize)]
struct CurrentKnowledgeVersion {
    #[serde(skip)]
    version_id: i64,
    #[serde(skip)]
    knowledge_entry_id: i64,
    version_public_id: Uuid,
    knowledge_public_id: Uuid,
    node_key: String,
    entry_type: String,
    title: String,
    statement: String,
    rationale: String,
}

#[derive(Debug, Clone, Serialize)]
struct CandidatePair {
    fingerprint: String,
    candidate_reason: String,
    shared_subject_terms: Vec<String>,
    left: CurrentKnowledgeVersion,
    right: CurrentKnowledgeVersion,
    #[serde(skip)]
    score: i32,
}

impl CandidatePair {
    fn public_ids(&self) -> (Uuid, Uuid) {
        canonical_pair(self.left.version_public_id, self.right.version_public_id)
    }
}

#[derive(Debug, Clone)]
struct ProjectSnapshot {
    project_id: i64,
    workspace_id: i64,
    project_public_id: Uuid,
    graph_version: i64,
    candidates: Vec<CandidatePair>,
    model_run_id: i64,
    model_run_public_id: Uuid,
}

#[derive(Debug, Clone, FromRow)]
struct DueStewardWorkspace {
    workspace_id: i64,
    workspace_public_id: Uuid,
    actor_id: Uuid,
    workspace_role: String,
}

#[derive(Debug, Clone, PartialEq)]
struct ValidatedAssessment {
    left_public_id: Uuid,
    right_public_id: Uuid,
    classification: String,
    severity: Option<String>,
    confidence: f64,
    title: String,
    explanation: String,
    fingerprint: String,
}

/// Analyzes all selected candidate pairs for a project snapshot.
///
/// `state` must already be scoped from an authenticated [`crate::auth::RequestContext`].
/// Call this only after the knowledge commit/revision transaction has committed.
pub async fn analyze_project(
    state: &AppState,
    project_public_id: Uuid,
    config: StewardConfig,
) -> AppResult<StewardRunResult> {
    let config = config.validate()?;
    ensure_mutating_role(state)?;

    let prepared = prepare_run(state, project_public_id, config).await?;
    let Some(snapshot) = prepared else {
        let graph_version = current_graph_version(state, project_public_id).await?;
        return Ok(StewardRunResult {
            project_public_id,
            source_graph_version: graph_version,
            model_run_public_id: None,
            candidate_pair_count: 0,
            assessment_count: 0,
            contradiction_count: 0,
            insight_public_ids: Vec::new(),
        });
    };

    let candidate_payload = serde_json::to_value(&snapshot.candidates)
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let generated = state
        .engine
        .analyze_contradictions(StewardInput {
            candidate_pairs: candidate_payload,
        })
        .await;

    let generated = match generated {
        Ok(generated) => generated,
        Err(error) => {
            record_failed_run(state, snapshot.model_run_id, &error).await?;
            return Err(error);
        }
    };

    let raw_output = serde_json::to_value(&generated.output)
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let assessments =
        match validate_assessments(&snapshot.candidates, &generated.output.assessments) {
            Ok(assessments) => assessments,
            Err(reason) => {
                let error = AppError::Agent(format!("invalid Steward structured output: {reason}"));
                record_invalid_run(
                    state,
                    snapshot.model_run_id,
                    &generated,
                    &raw_output,
                    &error,
                )
                .await?;
                return Err(error);
            }
        };

    persist_completed_run(state, snapshot, generated, raw_output, assessments).await
}

/// Processes a domain event that was previously claimed with [`Outbox::claim_batch`].
///
/// The lease is renewed before provider work and by a heartbeat while the
/// analysis runs. Success acknowledges the event. Failure records the model
/// failure when applicable, then schedules retry/dead-letter through `Outbox`.
pub async fn process_claimed_event(
    state: &AppState,
    outbox: &Outbox,
    event: &ClaimedDomainEvent,
    worker_id: &str,
    config: StewardConfig,
) -> AppResult<StewardEventOutcome> {
    let workspace_id = request_workspace_id(state)?;
    if event.workspace_id != workspace_id {
        return Err(AppError::Forbidden);
    }

    renew_event_lease(state, outbox, event, worker_id).await?;
    if !is_steward_event(&event.event_type) {
        acknowledge_event(state, outbox, event, worker_id).await?;
        return Ok(StewardEventOutcome::Ignored);
    }

    let project_id = event
        .project_id
        .ok_or_else(|| AppError::Invalid("Steward event has no project".into()))?;
    let project_public_id = project_public_id_by_internal(state, project_id).await?;

    let heartbeat_state = state.clone();
    let heartbeat_outbox = outbox.clone();
    let heartbeat_event = event.clone();
    let heartbeat_worker = worker_id.to_owned();
    let heartbeat_period = heartbeat_period(outbox.policy().lease_duration);
    let heartbeat = tokio::spawn(async move {
        let mut interval = tokio::time::interval(heartbeat_period);
        interval.tick().await;
        loop {
            interval.tick().await;
            if renew_event_lease(
                &heartbeat_state,
                &heartbeat_outbox,
                &heartbeat_event,
                &heartbeat_worker,
            )
            .await
            .is_err()
            {
                break;
            }
        }
    });

    let result = analyze_project(state, project_public_id, config).await;
    heartbeat.abort();

    match result {
        Ok(result) => {
            acknowledge_event(state, outbox, event, worker_id).await?;
            Ok(StewardEventOutcome::Assessed { result })
        }
        Err(error) => {
            fail_event(state, outbox, event, worker_id, &error).await?;
            Err(error)
        }
    }
}

/// Drains every currently available Steward event visible through the already
/// scoped application state.
///
/// Each iteration performs reclaim+claim in one short transaction, commits,
/// runs provider work without a database connection, and acknowledges or
/// reschedules in a new short transaction. The function stops as soon as no
/// eligible event remains; it never polls or sleeps waiting for future work.
///
/// A process crash after commit, a lost/full trigger channel, an expired lease,
/// or a future retry remains durable. The supervisor periodically discovers
/// only due Steward workspaces through a private allowlisted database function,
/// reconstructs an accepted owner/editor scope, and invokes this same drain
/// under request GUCs and forced RLS. No client request is required for recovery.
pub async fn drain_steward_outbox(state: &AppState) -> AppResult<StewardDrainSummary> {
    let outbox = Outbox::new(OutboxPolicy::default()).map_err(|error| map_outbox_error(&error))?;
    drain_steward_outbox_with(state, &outbox, StewardConfig::default()).await
}

async fn drain_steward_outbox_with(
    state: &AppState,
    outbox: &Outbox,
    config: StewardConfig,
) -> AppResult<StewardDrainSummary> {
    ensure_mutating_role(state)?;
    let workspace_id = request_workspace_id(state)?;
    let worker_id = format!("steward-{workspace_id}-{}", Uuid::new_v4());
    let event_types = STEWARD_EVENT_TYPES
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let mut summary = StewardDrainSummary::default();

    loop {
        let mut claim_tx = begin_request(state).await?;
        let reclaimed = outbox
            .reclaim_expired_leases(
                &mut claim_tx,
                workspace_id,
                &event_types,
                STEWARD_OUTBOX_BATCH_SIZE,
            )
            .await
            .map_err(|error| map_outbox_error(&error))?;
        let events = outbox
            .claim_batch(
                &mut claim_tx,
                workspace_id,
                &event_types,
                &worker_id,
                STEWARD_OUTBOX_BATCH_SIZE,
            )
            .await
            .map_err(|error| map_outbox_error(&error))?;
        claim_tx.commit().await?;

        summary.reclaimed_for_retry += reclaimed.retry_scheduled;
        summary.reclaimed_to_dead_letter += reclaimed.dead_lettered;
        if events.is_empty() {
            break;
        }
        summary.claimed += events.len();

        // One analysis of the newest claimed event covers the complete current
        // project graph. Earlier events for that same project are acknowledged
        // as coalesced, avoiding one provider call per knowledge item in a
        // multi-proposal commit.
        let latest_by_project = events
            .iter()
            .filter_map(|event| {
                event
                    .project_id
                    .map(|project_id| (project_id, event.sequence_id))
            })
            .fold(
                HashMap::<i64, i64>::new(),
                |mut latest, (project_id, sequence_id)| {
                    latest
                        .entry(project_id)
                        .and_modify(|current| *current = (*current).max(sequence_id))
                        .or_insert(sequence_id);
                    latest
                },
            );

        for event in events {
            let coalesced = event.project_id.is_some_and(|project_id| {
                latest_by_project
                    .get(&project_id)
                    .is_some_and(|latest| event.sequence_id < *latest)
            });
            if coalesced {
                match acknowledge_event(state, outbox, &event, &worker_id).await {
                    Ok(()) => {
                        summary.processed += 1;
                        summary.coalesced += 1;
                    }
                    Err(error) => {
                        summary.failed += 1;
                        tracing::warn!(
                            workspace_id,
                            event_public_id = %event.public_id,
                            error_code = error.public_code(),
                            "Steward could not acknowledge a coalesced outbox event; its lease will be reclaimed by a later drain"
                        );
                    }
                }
                continue;
            }

            match process_claimed_event(state, outbox, &event, &worker_id, config).await {
                Ok(StewardEventOutcome::Ignored) => {
                    summary.processed += 1;
                }
                Ok(StewardEventOutcome::Assessed { result }) => {
                    summary.processed += 1;
                    summary.insight_public_ids.extend(result.insight_public_ids);
                }
                Err(error) => {
                    // Provider/domain failures are already recorded through
                    // `mark_failed`. A lease/ack failure may instead remain
                    // processing until reclaim on a later scoped drain.
                    summary.failed += 1;
                    tracing::warn!(
                        workspace_id,
                        event_public_id = %event.public_id,
                        error_code = error.public_code(),
                        "Steward outbox event failed; retry scheduling or lease recovery will preserve it"
                    );
                }
            }
        }
    }

    summary.insight_public_ids.sort_unstable();
    summary.insight_public_ids.dedup();
    Ok(summary)
}

async fn supervise_drains(
    mut receiver: mpsc::Receiver<AppState>,
    outbox: Outbox,
    scanner_state: AppState,
    policy: StewardSupervisorPolicy,
) {
    let mut scan_interval = tokio::time::interval(policy.scan_interval);
    scan_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // Tokio intervals tick immediately once. Consume that tick so startup
    // recovery begins after one deliberate interval and shutdown can observe a
    // closed trigger channel without first touching the database.
    scan_interval.tick().await;

    loop {
        tokio::select! {
            scoped_state = receiver.recv() => {
                let Some(scoped_state) = scoped_state else {
                    break;
                };
                supervise_one_drain(scoped_state, &outbox, "post_commit").await;
            }
            _ = scan_interval.tick() => {
                match discover_due_steward_scopes(
                    &scanner_state,
                    policy.max_workspaces_per_scan,
                ).await {
                    Ok(scopes) => {
                        for scoped_state in scopes {
                            supervise_one_drain(scoped_state, &outbox, "periodic_recovery").await;
                        }
                    }
                    Err(error) => {
                        tracing::warn!(
                            error_code = error.public_code(),
                            "Steward recovery discovery failed closed; the next bounded scan will retry"
                        );
                    }
                }
            }
        }
    }
    tracing::info!("Steward outbox supervisor stopped after all triggers were released");
}

async fn discover_due_steward_scopes(
    scanner_state: &AppState,
    workspace_limit: u32,
) -> AppResult<Vec<AppState>> {
    let workspace_limit = i32::try_from(workspace_limit)
        .map_err(|_| AppError::Internal("Steward workspace scan limit overflowed".into()))?;
    let rows = sqlx::query_as::<_, DueStewardWorkspace>(
        "select workspace_id, workspace_public_id, actor_id, workspace_role
         from app.list_due_steward_workspaces($1)",
    )
    .bind(workspace_limit)
    .fetch_all(&scanner_state.pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            if row.workspace_id <= 0 || !matches!(row.workspace_role.as_str(), "owner" | "editor") {
                return Err(AppError::Internal(
                    "Steward recovery scanner returned an invalid workspace scope".into(),
                ));
            }
            Ok(scanner_state.scoped(&crate::auth::RequestContext {
                actor_id: row.actor_id,
                workspace_id: row.workspace_public_id,
                workspace_internal_id: Some(row.workspace_id),
                workspace_role: row.workspace_role,
            }))
        })
        .collect()
}

async fn supervise_one_drain(state: AppState, outbox: &Outbox, source: &'static str) {
    let workspace_id = state.workspace_internal_id;
    let worker_outbox = outbox.clone();
    let drain = tokio::spawn(async move {
        drain_steward_outbox_with(&state, &worker_outbox, StewardConfig::default()).await
    });
    match drain.await {
        Ok(Ok(summary)) => {
            tracing::info!(
                workspace_id,
                source,
                claimed = summary.claimed,
                processed = summary.processed,
                coalesced = summary.coalesced,
                failed = summary.failed,
                "Steward outbox drain completed"
            );
        }
        Ok(Err(error)) => {
            tracing::warn!(
                workspace_id,
                source,
                error_code = error.public_code(),
                "Steward outbox drain stopped; durable work will recover on a later bounded scan"
            );
        }
        Err(error) => {
            tracing::error!(
                workspace_id,
                source,
                %error,
                "Steward outbox drain panicked; the supervisor remains available"
            );
        }
    }
}

async fn prepare_run(
    state: &AppState,
    project_public_id: Uuid,
    config: StewardConfig,
) -> AppResult<Option<ProjectSnapshot>> {
    let mut tx = begin_request(state).await?;
    let project: Option<(i64, i64, i64)> = sqlx::query_as(
        "select id, workspace_id, graph_version
         from app.projects
         where public_id = $1 and workspace_id = $2",
    )
    .bind(project_public_id)
    .bind(request_workspace_id(state)?)
    .fetch_optional(&mut *tx)
    .await?;
    let (project_id, workspace_id, graph_version) = project.ok_or(AppError::NotFound)?;

    let versions = sqlx::query_as::<_, CurrentKnowledgeVersion>(
        "select version.id as version_id, entry.id as knowledge_entry_id,
                version.public_id as version_public_id,
                entry.public_id as knowledge_public_id,
                node.node_key, version.entry_type, version.title,
                version.statement, version.rationale
         from app.knowledge_entries entry
         join app.knowledge_entry_versions version
           on version.knowledge_entry_id = entry.id
          and version.version_number = entry.latest_version
          and version.project_id = entry.project_id
         join app.context_nodes node
           on node.id = version.context_node_id
          and node.project_id = version.project_id
         where entry.project_id = $1
           and entry.workspace_id = $2
           and entry.status = 'confirmed'
           and version.status = 'confirmed'
           and version.entry_type <> 'open_question'
         order by version.created_at desc, version.id desc
         limit $3",
    )
    .bind(project_id)
    .bind(workspace_id)
    .bind(i64::from(config.max_versions))
    .fetch_all(&mut *tx)
    .await?;
    let candidates = build_candidate_pairs(&versions, config.max_candidate_pairs);
    if candidates.is_empty() {
        tx.commit().await?;
        return Ok(None);
    }

    let source_public_ids = candidates
        .iter()
        .flat_map(|candidate| {
            [
                candidate.left.version_public_id,
                candidate.right.version_public_id,
            ]
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let input_hash = sha256_json(&json!({
        "project_public_id": project_public_id,
        "source_graph_version": graph_version,
        "candidate_pairs": candidates,
    }))?;
    let (model_run_id, model_run_public_id): (i64, Uuid) = sqlx::query_as(
        "insert into app.model_runs (
           workspace_id, project_id, operation, provider, model,
           prompt_version, schema_version, source_graph_version, input_hash,
           source_public_ids, status, attempt_count, started_at
         ) values ($1,$2,'assess_contradiction',$3,$4,$5,$6,$7,$8,$9,
                   'running',0,clock_timestamp())
         returning id, public_id",
    )
    .bind(workspace_id)
    .bind(project_id)
    .bind(state.engine.provider_name())
    .bind(state.engine.requested_model())
    .bind(STEWARD_PROMPT_VERSION)
    .bind(STEWARD_SCHEMA_VERSION)
    .bind(graph_version)
    .bind(input_hash)
    .bind(source_public_ids)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Some(ProjectSnapshot {
        project_id,
        workspace_id,
        project_public_id,
        graph_version,
        candidates,
        model_run_id,
        model_run_public_id,
    }))
}

async fn persist_completed_run(
    state: &AppState,
    snapshot: ProjectSnapshot,
    generated: EngineOutput<StewardOutput>,
    raw_output: Value,
    assessments: Vec<ValidatedAssessment>,
) -> AppResult<StewardRunResult> {
    let mut tx = begin_request(state).await?;
    let current_graph_version: Option<i64> = sqlx::query_scalar(
        "select graph_version from app.projects
         where id = $1 and workspace_id = $2
         for update",
    )
    .bind(snapshot.project_id)
    .bind(snapshot.workspace_id)
    .fetch_optional(&mut *tx)
    .await?;
    let current_graph_version = current_graph_version.ok_or(AppError::NotFound)?;

    if current_graph_version != snapshot.graph_version {
        complete_cancelled_run(
            &mut tx,
            snapshot.model_run_id,
            &generated.metadata,
            &raw_output,
        )
        .await?;
        tx.commit().await?;
        return Err(AppError::Conflict(format!(
            "project graph advanced from {} to {} during Steward analysis",
            snapshot.graph_version, current_graph_version
        )));
    }

    complete_model_run(
        &mut tx,
        snapshot.model_run_id,
        &generated.metadata,
        &raw_output,
    )
    .await?;

    let version_map = snapshot
        .candidates
        .iter()
        .flat_map(|candidate| [&candidate.left, &candidate.right])
        .map(|version| (version.version_public_id, version.version_id))
        .collect::<HashMap<_, _>>();
    let mut insight_public_ids = Vec::new();
    let mut contradiction_count = 0_usize;

    for assessment in &assessments {
        let confidence = format!("{:.3}", assessment.confidence);
        let assessment_id: Option<i64> = sqlx::query_scalar(
            "insert into app.steward_assessments (
               workspace_id, project_id, model_run_id, graph_version, fingerprint,
               classification, severity, confidence, title, explanation
             ) values ($1,$2,$3,$4,$5,$6,$7,$8::numeric,$9,$10)
             on conflict (project_id, fingerprint, graph_version) do nothing
             returning id",
        )
        .bind(snapshot.workspace_id)
        .bind(snapshot.project_id)
        .bind(snapshot.model_run_id)
        .bind(snapshot.graph_version)
        .bind(&assessment.fingerprint)
        .bind(&assessment.classification)
        .bind(&assessment.severity)
        .bind(&confidence)
        .bind(&assessment.title)
        .bind(&assessment.explanation)
        .fetch_optional(&mut *tx)
        .await?;
        let assessment_id = match assessment_id {
            Some(id) => id,
            None => {
                sqlx::query_scalar(
                    "select id from app.steward_assessments
                 where project_id = $1 and fingerprint = $2 and graph_version = $3",
                )
                .bind(snapshot.project_id)
                .bind(&assessment.fingerprint)
                .bind(snapshot.graph_version)
                .fetch_one(&mut *tx)
                .await?
            }
        };

        for (role, source_public_id) in [
            ("left", assessment.left_public_id),
            ("right", assessment.right_public_id),
        ] {
            let version_id = version_map.get(&source_public_id).copied().ok_or_else(|| {
                AppError::Internal("validated Steward source disappeared from snapshot".into())
            })?;
            sqlx::query(
                "insert into app.steward_assessment_sources (
                   workspace_id, project_id, assessment_id, source_role,
                   knowledge_entry_version_id
                 ) values ($1,$2,$3,$4,$5)
                 on conflict (assessment_id, source_role) do nothing",
            )
            .bind(snapshot.workspace_id)
            .bind(snapshot.project_id)
            .bind(assessment_id)
            .bind(role)
            .bind(version_id)
            .execute(&mut *tx)
            .await?;
        }

        if assessment.classification != "contradiction" {
            continue;
        }
        contradiction_count += 1;
        let severity = assessment
            .severity
            .as_deref()
            .ok_or_else(|| AppError::Internal("validated contradiction has no severity".into()))?;
        let insight_id: Option<(i64, Uuid)> = sqlx::query_as(
            "insert into app.insights (
               workspace_id, project_id, steward_assessment_id, insight_type,
               status, severity, confidence, title, explanation
             ) values ($1,$2,$3,'contradiction','open',$4,$5::numeric,$6,$7)
             on conflict (steward_assessment_id)
               where steward_assessment_id is not null do nothing
             returning id, public_id",
        )
        .bind(snapshot.workspace_id)
        .bind(snapshot.project_id)
        .bind(assessment_id)
        .bind(severity)
        .bind(&confidence)
        .bind(&assessment.title)
        .bind(&assessment.explanation)
        .fetch_optional(&mut *tx)
        .await?;
        let (insight_id, insight_public_id) = match insight_id {
            Some(row) => row,
            None => {
                sqlx::query_as(
                    "select id, public_id from app.insights
                 where steward_assessment_id = $1 and project_id = $2",
                )
                .bind(assessment_id)
                .bind(snapshot.project_id)
                .fetch_one(&mut *tx)
                .await?
            }
        };

        for (role, source_public_id) in [
            ("left", assessment.left_public_id),
            ("right", assessment.right_public_id),
        ] {
            let version_id = version_map[&source_public_id];
            sqlx::query(
                "insert into app.insight_sources (
                   workspace_id, project_id, insight_id, source_role, object_kind,
                   object_public_id, knowledge_entry_version_id
                 ) values ($1,$2,$3,$4,'knowledge_entry_version',$5,$6)
                 on conflict (insight_id, source_role, object_public_id) do nothing",
            )
            .bind(snapshot.workspace_id)
            .bind(snapshot.project_id)
            .bind(insight_id)
            .bind(role)
            .bind(source_public_id)
            .bind(version_id)
            .execute(&mut *tx)
            .await?;
        }
        insight_public_ids.push(insight_public_id);
    }

    tx.commit().await?;
    insight_public_ids.sort_unstable();
    insight_public_ids.dedup();
    Ok(StewardRunResult {
        project_public_id: snapshot.project_public_id,
        source_graph_version: snapshot.graph_version,
        model_run_public_id: Some(snapshot.model_run_public_id),
        candidate_pair_count: snapshot.candidates.len(),
        assessment_count: assessments.len(),
        contradiction_count,
        insight_public_ids,
    })
}

async fn complete_model_run(
    tx: &mut Transaction<'_, Postgres>,
    model_run_id: i64,
    metadata: &AgentRunMetadata,
    output: &Value,
) -> AppResult<()> {
    let input_tokens = metadata
        .input_tokens
        .and_then(|value| i32::try_from(value).ok());
    let output_tokens = metadata
        .output_tokens
        .and_then(|value| i32::try_from(value).ok());
    let latency_ms = i32::try_from(metadata.latency_ms).unwrap_or(i32::MAX);
    let model = metadata
        .served_model
        .as_deref()
        .unwrap_or(&metadata.requested_model);
    sqlx::query(
        "update app.model_runs
         set provider = $2, model = $3, provider_response_id = $4,
             status = 'completed', output = $5, usage = $6,
             input_tokens = $7, output_tokens = $8,
             estimated_cost = $9::double precision::numeric, latency_ms = $10,
             attempt_count = $11, error_class = null, error_message = null,
             completed_at = clock_timestamp()
         where id = $1 and status = 'running'",
    )
    .bind(model_run_id)
    .bind(&metadata.provider)
    .bind(model)
    .bind(&metadata.provider_response_id)
    .bind(output)
    .bind(json!({
        "input_tokens": metadata.input_tokens,
        "output_tokens": metadata.output_tokens,
        "provider_request_id": metadata.provider_request_id,
        "provider_status": metadata.status,
        "estimated_cost_usd": metadata.estimated_cost,
    }))
    .bind(input_tokens)
    .bind(output_tokens)
    .bind(metadata.estimated_cost)
    .bind(latency_ms)
    .bind(metadata.attempts.max(1))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn complete_cancelled_run(
    tx: &mut Transaction<'_, Postgres>,
    model_run_id: i64,
    metadata: &AgentRunMetadata,
    output: &Value,
) -> AppResult<()> {
    let input_tokens = metadata
        .input_tokens
        .and_then(|value| i32::try_from(value).ok());
    let output_tokens = metadata
        .output_tokens
        .and_then(|value| i32::try_from(value).ok());
    let latency_ms = i32::try_from(metadata.latency_ms).unwrap_or(i32::MAX);
    let model = metadata
        .served_model
        .as_deref()
        .unwrap_or(&metadata.requested_model);
    sqlx::query(
        "update app.model_runs
         set provider = $2, model = $3, provider_response_id = $4,
             status = 'cancelled', output = $5, usage = $6,
             input_tokens = $7, output_tokens = $8,
             estimated_cost = $9::double precision::numeric, latency_ms = $10,
             attempt_count = $11, completed_at = clock_timestamp()
         where id = $1 and status = 'running'",
    )
    .bind(model_run_id)
    .bind(&metadata.provider)
    .bind(model)
    .bind(&metadata.provider_response_id)
    .bind(output)
    .bind(json!({
        "input_tokens": metadata.input_tokens,
        "output_tokens": metadata.output_tokens,
        "provider_request_id": metadata.provider_request_id,
        "provider_status": metadata.status,
        "estimated_cost_usd": metadata.estimated_cost,
        "cancel_reason": "graph_version_changed",
    }))
    .bind(input_tokens)
    .bind(output_tokens)
    .bind(metadata.estimated_cost)
    .bind(latency_ms)
    .bind(metadata.attempts.max(1))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn record_failed_run(state: &AppState, model_run_id: i64, error: &AppError) -> AppResult<()> {
    let mut tx = begin_request(state).await?;
    sqlx::query(
        "update app.model_runs
         set status = 'failed', error_class = $2, error_message = $3,
             attempt_count = greatest(attempt_count, $4),
             latency_ms = coalesce(
               latency_ms,
               least(
                 2147483647::numeric,
                 greatest(
                   0::numeric,
                   floor(extract(epoch from (clock_timestamp() - started_at)) * 1000)
                 )
               )::integer
             ),
             completed_at = clock_timestamp()
         where id = $1 and status = 'running'",
    )
    .bind(model_run_id)
    .bind(error.model_run_error_class())
    .bind(bounded_error_message(&error.public_message()))
    .bind(provider_attempt_count(error))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn record_invalid_run(
    state: &AppState,
    model_run_id: i64,
    generated: &EngineOutput<StewardOutput>,
    raw_output: &Value,
    error: &AppError,
) -> AppResult<()> {
    let mut tx = begin_request(state).await?;
    let input_tokens = generated
        .metadata
        .input_tokens
        .and_then(|value| i32::try_from(value).ok());
    let output_tokens = generated
        .metadata
        .output_tokens
        .and_then(|value| i32::try_from(value).ok());
    let latency_ms = i32::try_from(generated.metadata.latency_ms).unwrap_or(i32::MAX);
    let model = generated
        .metadata
        .served_model
        .as_deref()
        .unwrap_or(&generated.metadata.requested_model);
    sqlx::query(
        "update app.model_runs
         set provider = $2, model = $3, provider_response_id = $4,
             status = 'failed', output = $5, usage = $6,
             input_tokens = $7, output_tokens = $8,
             estimated_cost = $9::double precision::numeric, latency_ms = $10,
             attempt_count = $11, error_class = 'invalid_structured_output',
             error_message = $12, completed_at = clock_timestamp()
         where id = $1 and status = 'running'",
    )
    .bind(model_run_id)
    .bind(&generated.metadata.provider)
    .bind(model)
    .bind(&generated.metadata.provider_response_id)
    .bind(raw_output)
    .bind(json!({
        "input_tokens": generated.metadata.input_tokens,
        "output_tokens": generated.metadata.output_tokens,
        "provider_request_id": generated.metadata.provider_request_id,
        "provider_status": generated.metadata.status,
        "estimated_cost_usd": generated.metadata.estimated_cost,
    }))
    .bind(input_tokens)
    .bind(output_tokens)
    .bind(generated.metadata.estimated_cost)
    .bind(latency_ms)
    .bind(generated.metadata.attempts.max(1))
    .bind(bounded_error_message(&error.public_message()))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

fn validate_assessments(
    candidates: &[CandidatePair],
    drafts: &[ContradictionAssessmentDraft],
) -> Result<Vec<ValidatedAssessment>, String> {
    if drafts.len() != candidates.len() {
        return Err(format!(
            "expected {} assessment(s), received {}",
            candidates.len(),
            drafts.len()
        ));
    }

    let candidate_map = candidates
        .iter()
        .map(|candidate| (candidate.public_ids(), candidate))
        .collect::<HashMap<_, _>>();
    let mut seen = HashSet::with_capacity(drafts.len());
    let mut validated = Vec::with_capacity(drafts.len());

    for draft in drafts {
        if draft.source_version_ids.len() != 2 {
            return Err("each assessment must cite exactly two source UUIDs".into());
        }
        let left = draft.source_version_ids[0];
        let right = draft.source_version_ids[1];
        if left == right {
            return Err("an assessment cannot cite the same source twice".into());
        }
        let pair = canonical_pair(left, right);
        let candidate = candidate_map.get(&pair).ok_or_else(|| {
            "assessment cites a pair outside the bounded candidate set".to_owned()
        })?;
        if !seen.insert(pair) {
            return Err("the same candidate pair was assessed more than once".into());
        }

        let classification = match draft.verdict.as_str() {
            "contradiction" | "compatible" | "ambiguous" => draft.verdict.clone(),
            _ => return Err(format!("unsupported verdict: {}", draft.verdict)),
        };
        let reported_severity = match draft.severity.as_str() {
            "info" | "notice" => "notice",
            "warning" => "warning",
            "blocking" => "blocking",
            _ => return Err(format!("unsupported severity: {}", draft.severity)),
        };
        let severity = (classification == "contradiction").then(|| reported_severity.to_owned());
        if !draft.confidence.is_finite() || !(0.0..=1.0).contains(&draft.confidence) {
            return Err("confidence must be a finite number between zero and one".into());
        }
        let title = validate_text(&draft.title, "title", MAX_TITLE_CHARS)?;
        let explanation = validate_text(&draft.explanation, "explanation", MAX_EXPLANATION_CHARS)?;

        validated.push(ValidatedAssessment {
            left_public_id: candidate.left.version_public_id,
            right_public_id: candidate.right.version_public_id,
            classification,
            severity,
            confidence: draft.confidence,
            title,
            explanation,
            fingerprint: candidate.fingerprint.clone(),
        });
    }
    validated.sort_by_key(|assessment| {
        canonical_pair(assessment.left_public_id, assessment.right_public_id)
    });
    Ok(validated)
}

fn validate_text(value: &str, field: &str, max_chars: usize) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field} must not be blank"));
    }
    if trimmed.chars().count() > max_chars {
        return Err(format!("{field} exceeds {max_chars} characters"));
    }
    Ok(trimmed.to_owned())
}

fn build_candidate_pairs(
    versions: &[CurrentKnowledgeVersion],
    max_pairs: usize,
) -> Vec<CandidatePair> {
    let mut pairs = Vec::new();
    for (left_index, left) in versions.iter().enumerate() {
        for right in versions.iter().skip(left_index + 1) {
            if left.knowledge_entry_id == right.knowledge_entry_id {
                continue;
            }
            let left_terms = subject_terms(&format!("{} {}", left.title, left.statement));
            let right_terms = subject_terms(&format!("{} {}", right.title, right.statement));
            let shared_subject_terms = left_terms
                .intersection(&right_terms)
                .take(8)
                .cloned()
                .collect::<Vec<_>>();
            let cross_scope = left.node_key != right.node_key;
            let type_affinity = entry_type_affinity(&left.entry_type, &right.entry_type);
            if shared_subject_terms.is_empty() && type_affinity == 0 {
                continue;
            }

            let score = i32::try_from(shared_subject_terms.len()).unwrap_or(i32::MAX) * 10
                + i32::from(cross_scope) * 5
                + type_affinity;
            let mut reason_codes = Vec::new();
            if !shared_subject_terms.is_empty() {
                reason_codes.push("shared_subject");
            }
            if cross_scope {
                reason_codes.push("cross_scope");
            }
            if type_affinity > 0 {
                reason_codes.push("type_affinity");
            }
            let (left, right) = if left.version_public_id <= right.version_public_id {
                (left.clone(), right.clone())
            } else {
                (right.clone(), left.clone())
            };
            pairs.push(CandidatePair {
                fingerprint: pair_fingerprint(left.version_public_id, right.version_public_id),
                candidate_reason: reason_codes.join("+"),
                shared_subject_terms,
                left,
                right,
                score,
            });
        }
    }
    pairs.sort_by_key(|pair| {
        (
            Reverse(pair.score),
            pair.left.version_public_id,
            pair.right.version_public_id,
        )
    });
    pairs.truncate(max_pairs);
    pairs
}

fn entry_type_affinity(left: &str, right: &str) -> i32 {
    let pair = if left <= right {
        (left, right)
    } else {
        (right, left)
    };
    match pair {
        ("business_rule", "technical_rule") => 9,
        ("business_rule", "constraint") | ("constraint", "technical_rule") => 8,
        ("decision", "technical_rule") | ("business_rule" | "constraint", "decision") => 7,
        ("requirement", "technical_rule") | ("constraint", "requirement") => 6,
        ("acceptance_criterion", "requirement") => 5,
        _ if left == right => 3,
        _ => 0,
    }
}

fn subject_terms(value: &str) -> BTreeSet<String> {
    const STOP_WORDS: &[&str] = &[
        "alors", "avec", "avoir", "cette", "comme", "dans", "doit", "doivent", "elle", "elles",
        "entre", "etre", "faire", "leurs", "lorsque", "pour", "sans", "sera", "sont", "sous",
        "tout", "toute", "toutes", "tous", "une", "version",
    ];
    value
        .to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|term| term.chars().count() >= 4 && !STOP_WORDS.contains(term))
        .map(str::to_owned)
        .collect()
}

fn canonical_pair(left: Uuid, right: Uuid) -> (Uuid, Uuid) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

fn pair_fingerprint(left: Uuid, right: Uuid) -> String {
    let (left, right) = canonical_pair(left, right);
    let mut digest = Sha256::new();
    digest.update(b"ai-center-steward-pair-v1\0");
    digest.update(left.as_bytes());
    digest.update(right.as_bytes());
    format!("{:x}", digest.finalize())
}

fn sha256_json<T: Serialize>(value: &T) -> AppResult<String> {
    let bytes = serde_json::to_vec(value).map_err(|error| AppError::Internal(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn ensure_mutating_role(state: &AppState) -> AppResult<()> {
    if matches!(state.workspace_role.as_str(), "owner" | "editor") {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

fn request_workspace_id(state: &AppState) -> AppResult<i64> {
    state
        .workspace_internal_id
        .ok_or_else(|| AppError::Internal("request-scoped workspace context is missing".into()))
}

async fn begin_request(state: &AppState) -> AppResult<Transaction<'_, Postgres>> {
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "select set_config('app.current_actor_id', $1, true),
                set_config('app.current_workspace_id', $2, true),
                set_config('app.current_workspace_role', $3, true)",
    )
    .bind(state.actor_id.to_string())
    .bind(request_workspace_id(state)?.to_string())
    .bind(&state.workspace_role)
    .execute(&mut *tx)
    .await?;
    Ok(tx)
}

async fn current_graph_version(state: &AppState, project_public_id: Uuid) -> AppResult<i64> {
    let mut tx = begin_request(state).await?;
    let graph_version: Option<i64> = sqlx::query_scalar(
        "select graph_version from app.projects
         where public_id = $1 and workspace_id = $2",
    )
    .bind(project_public_id)
    .bind(request_workspace_id(state)?)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    graph_version.ok_or(AppError::NotFound)
}

async fn project_public_id_by_internal(state: &AppState, project_id: i64) -> AppResult<Uuid> {
    let mut tx = begin_request(state).await?;
    let project_public_id: Option<Uuid> = sqlx::query_scalar(
        "select public_id from app.projects where id = $1 and workspace_id = $2",
    )
    .bind(project_id)
    .bind(request_workspace_id(state)?)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    project_public_id.ok_or(AppError::NotFound)
}

fn is_steward_event(event_type: &str) -> bool {
    matches!(event_type, "knowledge.committed" | "knowledge.revised")
}

fn heartbeat_period(lease_duration: Duration) -> Duration {
    let third = lease_duration / 3;
    third.clamp(Duration::from_secs(1), Duration::from_secs(20))
}

async fn renew_event_lease(
    state: &AppState,
    outbox: &Outbox,
    event: &ClaimedDomainEvent,
    worker_id: &str,
) -> AppResult<()> {
    let mut tx = begin_request(state).await?;
    outbox
        .renew_lease(&mut tx, event.public_id, worker_id, event.lease_attempt)
        .await
        .map_err(|error| map_outbox_error(&error))?;
    tx.commit().await?;
    Ok(())
}

async fn acknowledge_event(
    state: &AppState,
    outbox: &Outbox,
    event: &ClaimedDomainEvent,
    worker_id: &str,
) -> AppResult<()> {
    let mut tx = begin_request(state).await?;
    outbox
        .mark_processed(&mut tx, event.public_id, worker_id, event.lease_attempt)
        .await
        .map_err(|error| map_outbox_error(&error))?;
    tx.commit().await?;
    Ok(())
}

async fn fail_event(
    state: &AppState,
    outbox: &Outbox,
    event: &ClaimedDomainEvent,
    worker_id: &str,
    error: &AppError,
) -> AppResult<()> {
    let mut tx = begin_request(state).await?;
    outbox
        .mark_failed(
            &mut tx,
            event.public_id,
            worker_id,
            event.lease_attempt,
            error.public_code(),
            &error.public_message(),
        )
        .await
        .map_err(|error| map_outbox_error(&error))?;
    tx.commit().await?;
    Ok(())
}

fn map_outbox_error(error: &OutboxError) -> AppError {
    AppError::Internal(format!("Steward outbox transition failed: {error}"))
}

fn bounded_error_message(value: &str) -> String {
    value.chars().take(1_024).collect()
}

fn provider_attempt_count(error: &AppError) -> i32 {
    error
        .provider_attempts()
        .map_or(1, |attempts| i32::try_from(attempts).unwrap_or(i32::MAX))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sqlx::postgres::PgPoolOptions;

    use super::*;
    use crate::agent::DeterministicEngine;

    fn unscoped_scanner_state() -> AppState {
        AppState {
            pool: PgPoolOptions::new()
                .connect_lazy("postgresql://postgres:postgres@127.0.0.1/postgres")
                .expect("test database URL is valid"),
            engine: Arc::new(DeterministicEngine),
            workspace_id: Uuid::nil(),
            workspace_internal_id: None,
            workspace_role: "viewer".into(),
            actor_id: Uuid::nil(),
            agent_mode: "openai",
            steward_trigger: None,
        }
    }

    fn version(
        id: i64,
        public_id: Uuid,
        node_key: &str,
        entry_type: &str,
        title: &str,
        statement: &str,
    ) -> CurrentKnowledgeVersion {
        let uuid_seed = u128::try_from(id).expect("test version id must be positive");
        CurrentKnowledgeVersion {
            version_id: id,
            knowledge_entry_id: id,
            version_public_id: public_id,
            knowledge_public_id: Uuid::from_u128(10_000 + uuid_seed),
            node_key: node_key.into(),
            entry_type: entry_type.into(),
            title: title.into(),
            statement: statement.into(),
            rationale: String::new(),
        }
    }

    #[test]
    fn candidate_selection_is_bounded_and_deterministic() {
        let versions = (1..=12)
            .map(|id| {
                version(
                    id,
                    Uuid::from_u128(u128::try_from(id).expect("test version id must be positive")),
                    if id % 2 == 0 { "tech" } else { "product" },
                    if id % 3 == 0 {
                        "technical_rule"
                    } else {
                        "business_rule"
                    },
                    "Conservation du contexte partagé",
                    &format!("Le contexte partagé numéro {id} doit rester cohérent"),
                )
            })
            .collect::<Vec<_>>();
        let first = build_candidate_pairs(&versions, 7);
        let second = build_candidate_pairs(&versions, 7);

        assert_eq!(first.len(), 7);
        assert_eq!(
            first
                .iter()
                .map(CandidatePair::public_ids)
                .collect::<Vec<_>>(),
            second
                .iter()
                .map(CandidatePair::public_ids)
                .collect::<Vec<_>>()
        );
        assert!(first.windows(2).all(|pair| pair[0].score >= pair[1].score));
    }

    #[test]
    fn fingerprint_is_order_independent_and_version_specific() {
        let one = Uuid::from_u128(1);
        let two = Uuid::from_u128(2);
        let three = Uuid::from_u128(3);

        assert_eq!(pair_fingerprint(one, two), pair_fingerprint(two, one));
        assert_ne!(pair_fingerprint(one, two), pair_fingerprint(one, three));
        assert_eq!(pair_fingerprint(one, two).len(), 64);
    }

    #[test]
    fn validation_rejects_an_invented_source_pair() {
        let left = version(
            1,
            Uuid::from_u128(1),
            "product",
            "business_rule",
            "Contexte partagé",
            "Le contexte partagé reste disponible",
        );
        let right = version(
            2,
            Uuid::from_u128(2),
            "tech",
            "technical_rule",
            "Contexte partagé",
            "Le contexte partagé est archivé",
        );
        let candidates = build_candidate_pairs(&[left, right], 8);
        let drafts = vec![ContradictionAssessmentDraft {
            source_version_ids: vec![Uuid::from_u128(1), Uuid::from_u128(99)],
            verdict: "contradiction".into(),
            severity: "blocking".into(),
            confidence: 0.9,
            title: "Politique incompatible".into(),
            explanation: "Les deux politiques ne peuvent pas être appliquées ensemble.".into(),
        }];

        let error = validate_assessments(&candidates, &drafts).unwrap_err();
        assert!(error.contains("outside the bounded candidate set"));
    }

    #[test]
    fn validation_requires_one_assessment_per_candidate_without_duplicates() {
        let versions = vec![
            version(
                1,
                Uuid::from_u128(1),
                "product",
                "business_rule",
                "Contexte partagé",
                "Le contexte partagé reste disponible",
            ),
            version(
                2,
                Uuid::from_u128(2),
                "tech",
                "technical_rule",
                "Contexte partagé",
                "Le contexte partagé est archivé",
            ),
            version(
                3,
                Uuid::from_u128(3),
                "tech",
                "constraint",
                "Contexte partagé",
                "Le contexte partagé reste borné",
            ),
        ];
        let candidates = build_candidate_pairs(&versions, 8);
        assert!(candidates.len() > 1);
        let pair = candidates[0].public_ids();
        let duplicate = ContradictionAssessmentDraft {
            source_version_ids: vec![pair.0, pair.1],
            verdict: "compatible".into(),
            severity: "info".into(),
            confidence: 0.8,
            title: "Compatible".into(),
            explanation: "Les deux règles peuvent coexister.".into(),
        };
        let drafts = vec![duplicate.clone(); candidates.len()];

        let error = validate_assessments(&candidates, &drafts).unwrap_err();
        assert!(error.contains("more than once"));
    }

    #[test]
    fn validation_keeps_compatible_and_ambiguous_assessments_without_severity() {
        let left = version(
            1,
            Uuid::from_u128(1),
            "product",
            "business_rule",
            "Contexte partagé",
            "Le contexte partagé reste disponible",
        );
        let right = version(
            2,
            Uuid::from_u128(2),
            "tech",
            "technical_rule",
            "Contexte partagé",
            "Le contexte partagé est répliqué",
        );
        let candidates = build_candidate_pairs(&[left, right], 8);
        let pair = candidates[0].public_ids();
        let drafts = vec![ContradictionAssessmentDraft {
            source_version_ids: vec![pair.1, pair.0],
            verdict: "ambiguous".into(),
            severity: "warning".into(),
            confidence: 0.6,
            title: "Précision nécessaire".into(),
            explanation: "La portée doit être précisée avant de conclure.".into(),
        }];

        let validated = validate_assessments(&candidates, &drafts).unwrap();
        assert_eq!(validated.len(), 1);
        assert_eq!(validated[0].classification, "ambiguous");
        assert_eq!(validated[0].severity, None);
    }

    #[tokio::test]
    async fn supervisor_stops_naturally_after_the_last_trigger_is_dropped() {
        let (trigger, supervisor) = StewardDrainSupervisor::start(unscoped_scanner_state())
            .expect("default outbox policy is valid");
        drop(trigger);
        tokio::time::timeout(Duration::from_secs(1), supervisor.shutdown())
            .await
            .expect("closed supervisor channel must terminate without polling");
    }

    #[test]
    fn recovery_scan_policy_rejects_aggressive_or_unbounded_values() {
        assert!(
            StewardSupervisorPolicy {
                scan_interval: Duration::from_millis(999),
                max_workspaces_per_scan: 16,
            }
            .validate()
            .is_err()
        );
        assert!(
            StewardSupervisorPolicy {
                scan_interval: Duration::from_secs(15),
                max_workspaces_per_scan: 129,
            }
            .validate()
            .is_err()
        );
        assert!(StewardSupervisorPolicy::default().validate().is_ok());
    }
}
