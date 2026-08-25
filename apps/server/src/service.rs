#![allow(clippy::missing_errors_doc, clippy::too_many_lines)]

use std::{collections::HashSet, fmt::Write as _, sync::Arc};

use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    agent::{
        AgentEngine, AgentInput, AgentRunMetadata, ContextSelectionInput, TechnicalPlanDraft,
        TechnicalPlanInput,
    },
    context::{
        ContextCandidate, DEFAULT_CONTEXT_BUDGET_TOKENS,
        compile_context_pack as compile_context_projection,
    },
    error::{AppError, AppResult},
    idempotency::{self, IdempotencyLease},
    models::{
        CommitResult, CompileContextPack, ContextNode, ContextPackSelectionItemView,
        ContextPackSummary, CoverageItem, CoverageView, CreateHandoff, CreateProject,
        CreateSession, DecideProposals, DeliverableSummary, GateCounts, GateResult,
        GenerateTechnicalPlan, HandoffView, Health, HistoryEvent, InsightAction, InsightDecision,
        InsightDetail, InsightResolutionMutation, InsightSourceView, InsightSummary,
        KnowledgeSummary, MessageView, ProjectSnapshot, ProjectSummary, ProposalDecision,
        ProposalView, ResolveInsight, ReviseKnowledge, RevisionResult, SendMessage, SessionSummary,
        SessionView, WorkspaceSummary,
    },
};

const EXTRACT_KNOWLEDGE_PROMPT_VERSION: &str = "alpha-agent-turn-v1";
const EXTRACT_KNOWLEDGE_SCHEMA_VERSION: &str = "alpha-agent-turn-v1";
const CONTEXT_SELECTION_PROMPT_VERSION: &str = "alpha-context-selection-v1";
const CONTEXT_SELECTION_SCHEMA_VERSION: &str = "alpha-context-selection-v1";
const TECHNICAL_PLAN_PROMPT_VERSION: &str = "alpha-technical-plan-v1";
const TECHNICAL_PLAN_SCHEMA_VERSION: &str = "alpha-technical-plan-v1";

#[derive(Clone, Copy, Debug, sqlx::FromRow)]
struct ModelRunHandle {
    id: i64,
    public_id: Uuid,
}

struct ModelRunStart<'a> {
    workspace_id: i64,
    project_id: i64,
    session_id: Option<i64>,
    context_pack_id: Option<i64>,
    operation: &'a str,
    prompt_version: &'a str,
    schema_version: &'a str,
    source_graph_version: i64,
    input_hash: &'a str,
    source_public_ids: &'a [Uuid],
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub engine: Arc<dyn AgentEngine>,
    pub workspace_id: Uuid,
    pub workspace_internal_id: Option<i64>,
    pub workspace_role: String,
    pub actor_id: Uuid,
    pub agent_mode: &'static str,
    /// Present in `OpenAI` mode. Deterministic tests drain the same outbox
    /// synchronously so they can assert the resulting insights.
    pub steward_trigger: Option<crate::steward::StewardDrainTrigger>,
}

impl AppState {
    /// Returns a request-scoped service state. The database pool and provider
    /// are shared, while every query observes the authenticated actor and
    /// workspace selected for this request.
    #[must_use]
    pub fn scoped(&self, context: &crate::auth::RequestContext) -> Self {
        Self {
            pool: self.pool.clone(),
            engine: Arc::clone(&self.engine),
            workspace_id: context.workspace_id,
            workspace_internal_id: context.workspace_internal_id,
            workspace_role: context.workspace_role.clone(),
            actor_id: context.actor_id,
            agent_mode: self.agent_mode,
            steward_trigger: self.steward_trigger.clone(),
        }
    }

    async fn begin_request(&self) -> AppResult<Transaction<'_, Postgres>> {
        let workspace_id = self.workspace_internal_id.ok_or_else(|| {
            AppError::Internal("request-scoped workspace context is missing".into())
        })?;
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "select set_config('app.current_actor_id', $1, true),
                    set_config('app.current_workspace_id', $2, true),
                    set_config('app.current_workspace_role', $3, true)",
        )
        .bind(self.actor_id.to_string())
        .bind(workspace_id.to_string())
        .bind(&self.workspace_role)
        .execute(&mut *tx)
        .await?;
        Ok(tx)
    }

    async fn begin_actor_request(&self) -> AppResult<Transaction<'_, Postgres>> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("select set_config('app.current_actor_id', $1, true)")
            .bind(self.actor_id.to_string())
            .execute(&mut *tx)
            .await?;
        Ok(tx)
    }
}

/// Atomically attaches a successful HTTP result to the mutation transaction.
///
/// Route-backed commands pass a lease; direct service calls used by internal
/// workers and tests pass `None`. Keeping this write in the same `PostgreSQL`
/// transaction as the domain effect closes the crash window between a
/// successful mutation and its durable replay record.
async fn complete_command<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    lease: Option<&IdempotencyLease>,
    output: &T,
) -> AppResult<()> {
    if let Some(lease) = lease {
        let body =
            serde_json::to_value(output).map_err(|error| AppError::Internal(error.to_string()))?;
        idempotency::complete(tx, lease, 200, body).await?;
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct ProjectRecord {
    id: i64,
    workspace_id: i64,
    template_id: i64,
    public_id: Uuid,
    name: String,
    objective: String,
    summary: String,
    status: String,
    graph_version: i64,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl ProjectRecord {
    fn summary(self) -> ProjectSummary {
        ProjectSummary {
            public_id: self.public_id,
            name: self.name,
            objective: self.objective,
            summary: self.summary,
            status: self.status,
            graph_version: self.graph_version,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct SessionRecord {
    id: i64,
    workspace_id: i64,
    project_id: i64,
    context_node_id: i64,
    agent_profile_id: i64,
    context_pack_id: Option<i64>,
    public_id: Uuid,
    title: String,
    status: String,
    node_key: String,
    scope_kind: String,
    instructions: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow)]
struct ContextPackRecord {
    id: i64,
    public_id: Uuid,
    version: i32,
    status: String,
    source_graph_version: i64,
    compiler_version: String,
    selection_mode: String,
    content_hash: String,
    token_budget: i32,
    estimated_tokens: i32,
    compiled_at: chrono::DateTime<chrono::Utc>,
    invalidated_at: Option<chrono::DateTime<chrono::Utc>>,
    stale_reason: Option<String>,
    content: Value,
}

pub async fn health(state: &AppState) -> AppResult<Health> {
    sqlx::query_scalar::<_, i32>("select 1")
        .fetch_one(&state.pool)
        .await?;
    Ok(Health {
        status: "ok",
        service: "ai-center-server",
        database: "connected",
        agent_mode: state.agent_mode,
    })
}

pub async fn list_projects(state: &AppState) -> AppResult<Vec<ProjectSummary>> {
    let mut tx = state.begin_request().await?;
    let projects = sqlx::query_as::<_, ProjectSummary>(
        "select p.public_id, p.name, p.objective, p.summary, p.status, p.graph_version, p.created_at, p.updated_at
         from app.projects p join app.workspaces w on w.id = p.workspace_id
         where w.public_id = $1 and p.status = 'active'
         order by p.updated_at desc, p.id desc",
    )
    .bind(state.workspace_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(projects)
}

pub async fn list_workspaces(state: &AppState) -> AppResult<Vec<WorkspaceSummary>> {
    let mut tx = state.begin_actor_request().await?;
    let workspaces = sqlx::query_as::<_, WorkspaceSummary>(
        "select public_id, name, role from app.list_actor_workspaces()",
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(workspaces)
}

pub async fn create_project(state: &AppState, input: CreateProject) -> AppResult<ProjectSummary> {
    create_project_command(state, input, None).await
}

pub async fn create_project_idempotent(
    state: &AppState,
    input: CreateProject,
    lease: &IdempotencyLease,
) -> AppResult<ProjectSummary> {
    create_project_command(state, input, Some(lease)).await
}

async fn create_project_command(
    state: &AppState,
    input: CreateProject,
    lease: Option<&IdempotencyLease>,
) -> AppResult<ProjectSummary> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::Invalid("project name is required".into()));
    }

    let mut tx = state.begin_request().await?;
    let (workspace_id,): (i64,) =
        sqlx::query_as("select id from app.workspaces where public_id = $1")
            .bind(state.workspace_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(AppError::NotFound)?;
    let (template_id,): (i64,) = sqlx::query_as(
        "select id from app.project_templates where template_key = 'software-product-delivery'",
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::Internal("system template is missing".into()))?;

    let project = sqlx::query_as::<_, ProjectRecord>(
        "insert into app.projects (workspace_id, template_id, name, objective, created_by_actor_id)
         values ($1, $2, $3, $4, $5)
         returning id, workspace_id, template_id, public_id, name, objective, summary, status,
                   graph_version, created_at, updated_at",
    )
    .bind(workspace_id)
    .bind(template_id)
    .bind(name)
    .bind(input.objective.trim())
    .bind(state.actor_id)
    .fetch_one(&mut *tx)
    .await?;

    for (node_key, title, description, profile_key, entries, deliverables) in [
        (
            "product",
            "Produit",
            "Intention, règles métier, exigences et critères d’acceptation.",
            "product-agent",
            vec![
                "business_rule",
                "requirement",
                "acceptance_criterion",
                "open_question",
            ],
            vec!["feature-brief"],
        ),
        (
            "tech",
            "Tech",
            "Architecture, plan de livraison, risques, preuves et couverture.",
            "tech-agent",
            vec!["technical_rule", "decision", "constraint"],
            vec!["technical-delivery-plan"],
        ),
    ] {
        sqlx::query(
            "insert into app.context_nodes (
               workspace_id, project_id, agent_profile_id, node_key, title, description,
               preferred_entry_types, preferred_deliverable_types
             )
             select $1, $2, ap.id, $3, $4, $5, $6, $7
             from app.agent_profiles ap where ap.template_id = $8 and ap.profile_key = $9",
        )
        .bind(workspace_id)
        .bind(project.id)
        .bind(node_key)
        .bind(title)
        .bind(description)
        .bind(entries)
        .bind(deliverables)
        .bind(template_id)
        .bind(profile_key)
        .execute(&mut *tx)
        .await?;
    }

    audit(
        &mut tx,
        workspace_id,
        Some(project.id),
        state.actor_id,
        "project.created",
        "project",
        project.public_id,
        None,
        Some(json!({"name": project.name, "objective": project.objective})),
    )
    .await?;
    let result = project.summary();
    complete_command(&mut tx, lease, &result).await?;
    tx.commit().await?;
    Ok(result)
}

pub async fn snapshot(state: &AppState, project_id: Uuid) -> AppResult<ProjectSnapshot> {
    let mut tx = state.begin_request().await?;
    let project = project_by_public(&mut tx, state.workspace_id, project_id).await?;
    let nodes = sqlx::query_as::<_, ContextNode>(
        "select n.public_id, n.node_key, n.title, n.description, n.summary,
                ap.profile_key, ap.name as profile_name, ap.scope_kind
         from app.context_nodes n join app.agent_profiles ap on ap.id = n.agent_profile_id
         where n.project_id = $1 order by n.id",
    )
    .bind(project.id)
    .fetch_all(&mut *tx)
    .await?;
    let sessions = sqlx::query_as::<_, SessionSummary>(
        "select s.public_id, p.public_id as project_public_id, s.title, s.status,
                n.node_key, ap.scope_kind, s.created_at, s.updated_at
         from app.sessions s
         join app.projects p on p.id = s.project_id
         join app.context_nodes n on n.id = s.context_node_id
         join app.agent_profiles ap on ap.id = s.agent_profile_id
         where s.project_id = $1 order by s.updated_at desc, s.id desc",
    )
    .bind(project.id)
    .fetch_all(&mut *tx)
    .await?;
    let knowledge = knowledge_for_project(&mut tx, project.id).await?;
    let deliverables = sqlx::query_as::<_, DeliverableSummary>(
        "select public_id, deliverable_type, title, summary, content, status, coverage_status, version, updated_at
         from app.deliverables where project_id = $1 order by updated_at desc, id desc",
    )
    .bind(project.id)
    .fetch_all(&mut *tx)
    .await?;
    let insights = insights_for_project(&mut tx, project.id).await?;
    let gate = latest_gate(&mut tx, project.id).await?;
    let latest_handoff = latest_handoff_by_project_id(&mut tx, project.id).await?;
    let snapshot = ProjectSnapshot {
        project: project.summary(),
        nodes,
        sessions,
        knowledge,
        deliverables,
        insights,
        gate,
        latest_handoff,
    };
    tx.commit().await?;
    Ok(snapshot)
}

pub async fn create_session(
    state: &AppState,
    project_public_id: Uuid,
    input: CreateSession,
) -> AppResult<SessionView> {
    create_session_command(state, project_public_id, input, None).await
}

pub async fn create_session_idempotent(
    state: &AppState,
    project_public_id: Uuid,
    input: CreateSession,
    lease: &IdempotencyLease,
) -> AppResult<SessionView> {
    create_session_command(state, project_public_id, input, Some(lease)).await
}

async fn create_session_command(
    state: &AppState,
    project_public_id: Uuid,
    input: CreateSession,
    lease: Option<&IdempotencyLease>,
) -> AppResult<SessionView> {
    let mut tx = state.begin_request().await?;
    let project = project_by_public(&mut tx, state.workspace_id, project_public_id).await?;
    if input.node_key == "tech" {
        return Err(AppError::Invalid(
            "a Tech session requires a current ContextPack and must be created through a handoff"
                .into(),
        ));
    }
    let (node_id, agent_profile_id, default_title): (i64, i64, String) = sqlx::query_as(
        "select n.id, n.agent_profile_id, n.title
         from app.context_nodes n where n.project_id = $1 and n.node_key = $2",
    )
    .bind(project.id)
    .bind(&input.node_key)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    let title = input
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map_or_else(|| format!("Session {default_title}"), str::to_owned);
    let public_id: Uuid = sqlx::query_scalar(
        "insert into app.sessions (
           workspace_id, project_id, context_node_id, agent_profile_id, title, created_by_actor_id
         ) values ($1, $2, $3, $4, $5, $6) returning public_id",
    )
    .bind(project.workspace_id)
    .bind(project.id)
    .bind(node_id)
    .bind(agent_profile_id)
    .bind(title)
    .bind(state.actor_id)
    .fetch_one(&mut *tx)
    .await?;
    let result = session_view_in_transaction(&mut tx, state.workspace_id, public_id).await?;
    complete_command(&mut tx, lease, &result).await?;
    tx.commit().await?;
    Ok(result)
}

pub async fn session_view(state: &AppState, public_id: Uuid) -> AppResult<SessionView> {
    let mut tx = state.begin_request().await?;
    let view = session_view_in_transaction(&mut tx, state.workspace_id, public_id).await?;
    tx.commit().await?;
    Ok(view)
}

async fn session_view_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    public_id: Uuid,
) -> AppResult<SessionView> {
    let session = session_by_public(tx, workspace_id, public_id).await?;
    let messages = sqlx::query_as::<_, MessageView>(
        "select public_id, role, content, agent_scope, metadata, created_at
         from app.messages where session_id = $1 order by created_at, id",
    )
    .bind(session.id)
    .fetch_all(&mut **tx)
    .await?;
    let proposals = sqlx::query_as::<_, ProposalView>(
        "select public_id, entry_type, title, statement, rationale, status, source_data, created_at
         from app.mutation_proposals where session_id = $1 order by created_at, id",
    )
    .bind(session.id)
    .fetch_all(&mut **tx)
    .await?;
    let context_pack = match session.context_pack_id {
        Some(id) => Some(context_pack_by_id(tx, id).await?),
        None => None,
    };
    let project_public_id = project_public_id_for_internal(tx, session.project_id).await?;
    let view = SessionView {
        session: SessionSummary {
            public_id: session.public_id,
            project_public_id,
            title: session.title,
            status: session.status,
            node_key: session.node_key,
            scope_kind: session.scope_kind,
            created_at: session.created_at,
            updated_at: session.updated_at,
        },
        messages,
        proposals,
        context_pack,
    };
    Ok(view)
}

pub async fn session_view_for_project(
    state: &AppState,
    project_public_id: Uuid,
    session_public_id: Uuid,
) -> AppResult<SessionView> {
    ensure_session_project(state, project_public_id, session_public_id).await?;
    session_view(state, session_public_id).await
}

pub async fn send_message_for_project(
    state: &AppState,
    project_public_id: Uuid,
    session_public_id: Uuid,
    input: SendMessage,
) -> AppResult<SessionView> {
    ensure_session_project(state, project_public_id, session_public_id).await?;
    send_message_command(state, session_public_id, input, None).await
}

pub async fn send_message_for_project_idempotent(
    state: &AppState,
    project_public_id: Uuid,
    session_public_id: Uuid,
    input: SendMessage,
    lease: &IdempotencyLease,
) -> AppResult<SessionView> {
    ensure_session_project(state, project_public_id, session_public_id).await?;
    send_message_command(state, session_public_id, input, Some(lease)).await
}

pub async fn decide_proposals_for_project(
    state: &AppState,
    project_public_id: Uuid,
    session_public_id: Uuid,
    input: DecideProposals,
) -> AppResult<CommitResult> {
    ensure_session_project(state, project_public_id, session_public_id).await?;
    decide_proposals_command(state, session_public_id, input, None).await
}

pub async fn decide_proposals_for_project_idempotent(
    state: &AppState,
    project_public_id: Uuid,
    session_public_id: Uuid,
    input: DecideProposals,
    lease: &IdempotencyLease,
) -> AppResult<CommitResult> {
    ensure_session_project(state, project_public_id, session_public_id).await?;
    decide_proposals_command(state, session_public_id, input, Some(lease)).await
}

pub async fn send_message(
    state: &AppState,
    session_id: Uuid,
    input: SendMessage,
) -> AppResult<SessionView> {
    send_message_command(state, session_id, input, None).await
}

async fn send_message_command(
    state: &AppState,
    session_id: Uuid,
    input: SendMessage,
    lease: Option<&IdempotencyLease>,
) -> AppResult<SessionView> {
    let content = input.content.trim();
    if content.is_empty() {
        return Err(AppError::Invalid("message content is required".into()));
    }
    let mut initial_tx = state.begin_request().await?;
    let session = session_by_public(&mut initial_tx, state.workspace_id, session_id).await?;

    // Persist the user's intent before contacting an external provider.
    let inserted_user = sqlx::query(
        "insert into app.messages (
           workspace_id, project_id, session_id, client_message_id, role, content
         ) values ($1, $2, $3, $4, 'user', $5)
         on conflict do nothing",
    )
    .bind(session.workspace_id)
    .bind(session.project_id)
    .bind(session.id)
    .bind(input.client_message_id)
    .bind(content)
    .execute(&mut *initial_tx)
    .await?;
    if inserted_user.rows_affected() == 0 {
        let assistant_exists: bool = sqlx::query_scalar(
            "select exists(select 1 from app.messages
             where session_id = $1 and role = 'assistant' and client_message_id = $2)",
        )
        .bind(session.id)
        .bind(input.client_message_id)
        .fetch_one(&mut *initial_tx)
        .await?;
        if assistant_exists {
            let result =
                session_view_in_transaction(&mut initial_tx, state.workspace_id, session_id)
                    .await?;
            complete_command(&mut initial_tx, lease, &result).await?;
            initial_tx.commit().await?;
            return Ok(result);
        }
    }

    let source_graph_version: i64 =
        sqlx::query_scalar("select graph_version from app.projects where id = $1")
            .bind(session.project_id)
            .fetch_one(&mut *initial_tx)
            .await?;
    let (knowledge_context, source_public_ids) = if session.scope_kind == "tech" {
        let pack_id = session.context_pack_id.ok_or_else(|| {
            AppError::Invalid("a Tech session requires an immutable ContextPack".into())
        })?;
        let pack = context_pack_by_id(&mut initial_tx, pack_id).await?;
        if pack.status != "current" || pack.source_graph_version != source_graph_version {
            return Err(AppError::Conflict(
                "the Tech session ContextPack is stale; recompile before continuing".into(),
            ));
        }
        let source_public_ids = sqlx::query_scalar(
            "select version.public_id
             from app.context_pack_sources source
             join app.knowledge_entry_versions version
               on version.id = source.knowledge_entry_version_id
             where source.context_pack_id = $1
             order by version.public_id",
        )
        .bind(pack_id)
        .fetch_all(&mut *initial_tx)
        .await?;
        (pack.content, source_public_ids)
    } else {
        let knowledge = knowledge_for_project(&mut initial_tx, session.project_id).await?;
        let source_public_ids = knowledge
            .iter()
            .map(|item| item.version_public_id)
            .collect();
        (
            json!({
                "scope": session.scope_kind,
                "node": session.node_key,
                "knowledge": knowledge,
            }),
            source_public_ids,
        )
    };
    let run_fingerprint = json!({
        "scope_kind": session.scope_kind,
        "instructions": session.instructions,
        "user_message": content,
        "context": knowledge_context,
    });
    let input_hash = sha256_json(&run_fingerprint)?;
    let model_run = start_model_run(
        &mut initial_tx,
        state.engine.provider_name(),
        state.engine.requested_model(),
        ModelRunStart {
            workspace_id: session.workspace_id,
            project_id: session.project_id,
            session_id: Some(session.id),
            context_pack_id: session.context_pack_id,
            operation: "extract_knowledge",
            prompt_version: EXTRACT_KNOWLEDGE_PROMPT_VERSION,
            schema_version: EXTRACT_KNOWLEDGE_SCHEMA_VERSION,
            source_graph_version,
            input_hash: &input_hash,
            source_public_ids: &source_public_ids,
        },
    )
    .await?;
    initial_tx.commit().await?;
    let engine_result = match state
        .engine
        .respond(AgentInput {
            scope_kind: session.scope_kind.clone(),
            instructions: session.instructions,
            user_message: content.into(),
            context: knowledge_context,
        })
        .await
    {
        Ok(result) => result,
        Err(error) => {
            record_failed_model_run(state, model_run.id, &error).await?;
            return Err(error);
        }
    };
    let turn = engine_result.output;
    let turn_output = match serde_json::to_value(&turn) {
        Ok(output) => output,
        Err(error) => {
            let error = AppError::Internal(error.to_string());
            record_failed_model_run_with_output(
                state,
                model_run.id,
                &engine_result.metadata,
                None,
                &error,
            )
            .await?;
            return Err(error);
        }
    };
    if let Err(error) = ensure_authorized_sources(&turn.sources, &source_public_ids) {
        record_failed_model_run_with_output(
            state,
            model_run.id,
            &engine_result.metadata,
            Some(&turn_output),
            &error,
        )
        .await?;
        return Err(error);
    }

    let mut tx = state.begin_request().await?;
    let finalization_conflict = if session.scope_kind == "tech" {
        let pack_id = session.context_pack_id.ok_or_else(|| {
            AppError::Invalid("a Tech session requires an immutable ContextPack".into())
        })?;
        let (current_graph_version, current_pack_status): (i64, String) = sqlx::query_as(
            "select project.graph_version, pack.status
             from app.projects project
             join app.context_packs pack on pack.project_id = project.id
             where project.id = $1 and pack.id = $2
             for update of project, pack",
        )
        .bind(session.project_id)
        .bind(pack_id)
        .fetch_one(&mut *tx)
        .await?;
        (current_graph_version != source_graph_version || current_pack_status != "current")
            .then(|| {
                AppError::Conflict(
                    "the Tech session ContextPack became stale while the provider response was in flight"
                        .into(),
                )
            })
    } else {
        let current_graph_version: i64 =
            sqlx::query_scalar("select graph_version from app.projects where id = $1 for update")
                .bind(session.project_id)
                .fetch_one(&mut *tx)
                .await?;
        (current_graph_version != source_graph_version).then(|| {
            AppError::Conflict(
                "the project context changed while the provider response was in flight".into(),
            )
        })
    };
    if let Some(error) = finalization_conflict {
        fail_model_run_in_transaction(
            &mut tx,
            model_run.id,
            Some(&engine_result.metadata),
            Some(&turn_output),
            &error,
        )
        .await?;
        tx.commit().await?;
        return Err(error);
    }
    let assistant_id: Option<i64> = sqlx::query_scalar(
        "insert into app.messages (
           workspace_id, project_id, session_id, client_message_id, role, content,
           agent_scope, metadata
         ) values ($1, $2, $3, $4, 'assistant', $5, $6, $7)
         on conflict do nothing returning id",
    )
    .bind(session.workspace_id)
    .bind(session.project_id)
    .bind(session.id)
    .bind(input.client_message_id)
    .bind(turn.response)
    .bind(&session.scope_kind)
    .bind(json!({
        "client_message_id": input.client_message_id,
        "sources": turn.sources,
        "model_run_public_id": model_run.public_id,
        "model_run": engine_result.metadata,
    }))
    .fetch_optional(&mut *tx)
    .await?;
    let Some(assistant_id) = assistant_id else {
        let error = AppError::Conflict(
            "an assistant response already exists for this client message".into(),
        );
        fail_model_run_in_transaction(
            &mut tx,
            model_run.id,
            Some(&engine_result.metadata),
            Some(&turn_output),
            &error,
        )
        .await?;
        let result = session_view_in_transaction(&mut tx, state.workspace_id, session_id).await?;
        complete_command(&mut tx, lease, &result).await?;
        tx.commit().await?;
        return Ok(result);
    };
    complete_model_run(
        &mut tx,
        model_run.id,
        session.context_pack_id,
        &engine_result.metadata,
        &turn_output,
    )
    .await?;
    for proposal in turn.proposals {
        sqlx::query(
            "insert into app.mutation_proposals (
               workspace_id, project_id, session_id, source_message_id, proposed_by_agent_profile_id,
               entry_type, title, statement, rationale, source_data
             ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(session.workspace_id)
        .bind(session.project_id)
        .bind(session.id)
        .bind(assistant_id)
        .bind(session.agent_profile_id)
        .bind(proposal.entry_type)
        .bind(proposal.title)
        .bind(proposal.statement)
        .bind(proposal.rationale)
        .bind(json!({"source_version_ids": turn.sources}))
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query("update app.sessions set updated_at = now() where id = $1")
        .bind(session.id)
        .execute(&mut *tx)
        .await?;
    let result = session_view_in_transaction(&mut tx, state.workspace_id, session_id).await?;
    complete_command(&mut tx, lease, &result).await?;
    tx.commit().await?;
    Ok(result)
}

fn ensure_authorized_sources(returned: &[Uuid], allowed: &[Uuid]) -> AppResult<()> {
    let allowed = allowed.iter().copied().collect::<HashSet<_>>();
    if returned.iter().all(|source| allowed.contains(source)) {
        Ok(())
    } else {
        Err(AppError::Agent(
            "provider returned a source outside the authorized project context".into(),
        ))
    }
}

#[derive(sqlx::FromRow)]
struct ProposalRecord {
    id: i64,
    public_id: Uuid,
    source_message_public_id: Uuid,
    entry_type: String,
    title: String,
    statement: String,
    rationale: String,
    status: String,
    source_data: Value,
}

pub async fn decide_proposals(
    state: &AppState,
    session_public_id: Uuid,
    input: DecideProposals,
) -> AppResult<CommitResult> {
    decide_proposals_command(state, session_public_id, input, None).await
}

async fn decide_proposals_command(
    state: &AppState,
    session_public_id: Uuid,
    input: DecideProposals,
    lease: Option<&IdempotencyLease>,
) -> AppResult<CommitResult> {
    if input.proposal_ids.is_empty() {
        return Err(AppError::Invalid(
            "at least one proposal is required".into(),
        ));
    }
    let mut tx = state.begin_request().await?;
    let session = session_by_public(&mut tx, state.workspace_id, session_public_id).await?;
    let proposals = sqlx::query_as::<_, ProposalRecord>(
        "select mp.id, mp.public_id, m.public_id as source_message_public_id, mp.entry_type,
                mp.title, mp.statement, mp.rationale, mp.status, mp.source_data
         from app.mutation_proposals mp join app.messages m on m.id = mp.source_message_id
         where mp.session_id = $1 and mp.public_id = any($2) for update of mp",
    )
    .bind(session.id)
    .bind(&input.proposal_ids)
    .fetch_all(&mut *tx)
    .await?;
    if proposals.len() != input.proposal_ids.len() {
        return Err(AppError::Invalid(
            "one or more proposals do not belong to this session".into(),
        ));
    }
    let desired_status = match input.decision {
        ProposalDecision::Confirm => "confirmed",
        ProposalDecision::Reject => "rejected",
    };
    if proposals
        .iter()
        .all(|proposal| proposal.status == desired_status)
    {
        let graph_version: i64 =
            sqlx::query_scalar("select graph_version from app.projects where id = $1")
                .bind(session.project_id)
                .fetch_one(&mut *tx)
                .await?;
        let confirmed = if desired_status == "confirmed" {
            sqlx::query_scalar::<_, Uuid>(
                "select distinct k.public_id
                 from app.knowledge_entries k
                 join app.knowledge_entry_versions v on v.knowledge_entry_id = k.id
                 where v.origin_type = 'agent_proposal' and v.origin_public_id = any($1)",
            )
            .bind(&input.proposal_ids)
            .fetch_all(&mut *tx)
            .await?
        } else {
            Vec::new()
        };
        let rejected = if desired_status == "rejected" {
            input.proposal_ids
        } else {
            Vec::new()
        };
        let mut result = CommitResult {
            confirmed,
            rejected,
            graph_version,
            insight_ids: Vec::new(),
        };
        complete_command(&mut tx, lease, &result).await?;
        tx.commit().await?;
        let insight_ids = if result.confirmed.is_empty() {
            Vec::new()
        } else {
            dispatch_steward_after_commit(state).await
        };
        if lease.is_none() {
            result.insight_ids = insight_ids;
        }
        return Ok(result);
    }
    if proposals
        .iter()
        .any(|proposal| proposal.status != "proposed")
    {
        return Err(AppError::Invalid(
            "a decided proposal cannot be decided again".into(),
        ));
    }

    let mut confirmed = Vec::new();
    let mut rejected = Vec::new();
    let graph_version = match input.decision {
        ProposalDecision::Confirm => sqlx::query_scalar::<_, i64>(
            "update app.projects set graph_version = graph_version + 1 where id = $1 returning graph_version",
        )
        .bind(session.project_id)
        .fetch_one(&mut *tx)
        .await?,
        ProposalDecision::Reject => sqlx::query_scalar::<_, i64>(
            "select graph_version from app.projects where id = $1",
        )
        .bind(session.project_id)
        .fetch_one(&mut *tx)
        .await?,
    };

    for proposal in proposals {
        match input.decision {
            ProposalDecision::Confirm => {
                let (knowledge_id, knowledge_public_id): (i64, Uuid) = sqlx::query_as(
                    "insert into app.knowledge_entries (
                       workspace_id, project_id, context_node_id, entry_type
                     ) values ($1,$2,$3,$4) returning id, public_id",
                )
                .bind(session.workspace_id)
                .bind(session.project_id)
                .bind(session.context_node_id)
                .bind(&proposal.entry_type)
                .fetch_one(&mut *tx)
                .await?;
                let version_public_id: Uuid = sqlx::query_scalar(
                    "insert into app.knowledge_entry_versions (
                       knowledge_entry_id, workspace_id, project_id, context_node_id, version_number,
                       entry_type, title, statement, rationale, author_actor_id, origin_type,
                       origin_public_id, source_message_public_id
                     ) values ($1,$2,$3,$4,1,$5,$6,$7,$8,$9,'agent_proposal',$10,$11)
                     returning public_id",
                )
                .bind(knowledge_id)
                .bind(session.workspace_id)
                .bind(session.project_id)
                .bind(session.context_node_id)
                .bind(&proposal.entry_type)
                .bind(&proposal.title)
                .bind(&proposal.statement)
                .bind(&proposal.rationale)
                .bind(state.actor_id)
                .bind(proposal.public_id)
                .bind(proposal.source_message_public_id)
                .fetch_one(&mut *tx)
                .await?;
                sqlx::query(
                    "update app.mutation_proposals
                     set status = 'confirmed', decided_by_actor_id = $2, decided_at = now()
                     where id = $1",
                )
                .bind(proposal.id)
                .bind(state.actor_id)
                .execute(&mut *tx)
                .await?;
                sqlx::query(
                    "insert into app.domain_events (
                       workspace_id, project_id, event_type, aggregate_kind, aggregate_public_id, payload
                     ) values ($1,$2,'knowledge.committed','knowledge_entry',$3,$4)",
                )
                .bind(session.workspace_id)
                .bind(session.project_id)
                .bind(knowledge_public_id)
                .bind(json!({"version_public_id": version_public_id, "graph_version": graph_version}))
                .execute(&mut *tx)
                .await?;
                for source_version_id in proposal
                    .source_data
                    .get("source_version_ids")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .filter_map(|value| value.parse::<Uuid>().ok())
                {
                    let inserted_source_edge = sqlx::query(
                        "insert into app.edges (
                           workspace_id, project_id, source_kind, source_public_id,
                           target_kind, target_public_id, edge_type, status, provenance,
                           created_by_actor_id
                         )
                         select $1,$2,'knowledge_entry_version',$3,
                           'knowledge_entry_version',source.public_id,
                           'derived_from','confirmed',$5,$6
                         from app.knowledge_entry_versions source
                         where source.public_id = $4
                           and source.project_id = $2
                           and source.workspace_id = $1
                         on conflict (project_id, source_public_id, target_public_id, edge_type)
                         do nothing",
                    )
                    .bind(session.workspace_id)
                    .bind(session.project_id)
                    .bind(version_public_id)
                    .bind(source_version_id)
                    .bind(json!({"proposal_public_id": proposal.public_id}))
                    .bind(state.actor_id)
                    .execute(&mut *tx)
                    .await?;
                    if inserted_source_edge.rows_affected() == 0 {
                        let source_exists_in_project: bool = sqlx::query_scalar(
                            "select exists(
                               select 1 from app.knowledge_entry_versions source
                               where source.public_id = $1
                                 and source.project_id = $2
                                 and source.workspace_id = $3
                             )",
                        )
                        .bind(source_version_id)
                        .bind(session.project_id)
                        .bind(session.workspace_id)
                        .fetch_one(&mut *tx)
                        .await?;
                        if !source_exists_in_project {
                            return Err(AppError::Conflict(
                                "proposal references a source outside the project context".into(),
                            ));
                        }
                    }
                }
                audit(
                    &mut tx,
                    session.workspace_id,
                    Some(session.project_id),
                    state.actor_id,
                    "knowledge.committed",
                    "knowledge_entry",
                    knowledge_public_id,
                    None,
                    Some(json!({
                        "version_public_id": version_public_id,
                        "entry_type": proposal.entry_type,
                        "title": proposal.title,
                        "statement": proposal.statement,
                        "graph_version": graph_version,
                    })),
                )
                .await?;
                confirmed.push(knowledge_public_id);
            }
            ProposalDecision::Reject => {
                sqlx::query(
                    "update app.mutation_proposals
                     set status = 'rejected', decided_by_actor_id = $2, decided_at = now()
                     where id = $1",
                )
                .bind(proposal.id)
                .bind(state.actor_id)
                .execute(&mut *tx)
                .await?;
                audit(
                    &mut tx,
                    session.workspace_id,
                    Some(session.project_id),
                    state.actor_id,
                    "proposal.rejected",
                    "mutation_proposal",
                    proposal.public_id,
                    Some(json!({"status": "proposed"})),
                    Some(json!({"status": "rejected"})),
                )
                .await?;
                rejected.push(proposal.public_id);
            }
        }
    }

    let should_analyze = !confirmed.is_empty();
    if should_analyze {
        invalidate_projections(&mut tx, session.project_id).await?;
    }
    let mut result = CommitResult {
        confirmed,
        rejected,
        graph_version,
        insight_ids: Vec::new(),
    };
    complete_command(&mut tx, lease, &result).await?;
    tx.commit().await?;
    let insight_ids = if should_analyze {
        dispatch_steward_after_commit(state).await
    } else {
        Vec::new()
    };
    if lease.is_none() {
        result.insight_ids = insight_ids;
    }
    Ok(result)
}

pub async fn evaluate_product_gate(
    state: &AppState,
    project_public_id: Uuid,
) -> AppResult<GateResult> {
    evaluate_product_gate_command(state, project_public_id, None).await
}

pub async fn evaluate_product_gate_idempotent(
    state: &AppState,
    project_public_id: Uuid,
    lease: &IdempotencyLease,
) -> AppResult<GateResult> {
    evaluate_product_gate_command(state, project_public_id, Some(lease)).await
}

async fn evaluate_product_gate_command(
    state: &AppState,
    project_public_id: Uuid,
    lease: Option<&IdempotencyLease>,
) -> AppResult<GateResult> {
    let mut tx = state.begin_request().await?;
    let project = project_by_public(&mut tx, state.workspace_id, project_public_id).await?;
    let (business_rules, requirements, acceptance_criteria, open_questions): (i64, i64, i64, i64) =
        sqlx::query_as(
            "select
               count(*) filter (where v.entry_type = 'business_rule'),
               count(*) filter (where v.entry_type = 'requirement'),
               count(*) filter (where v.entry_type = 'acceptance_criterion'),
               count(*) filter (where v.entry_type = 'open_question')
             from app.knowledge_entries k
             join app.knowledge_entry_versions v
               on v.knowledge_entry_id = k.id and v.version_number = k.latest_version
             join app.context_nodes n on n.id = k.context_node_id
             where k.project_id = $1 and n.node_key = 'product'
               and k.status = 'confirmed' and v.status = 'confirmed'",
        )
        .bind(project.id)
        .fetch_one(&mut *tx)
        .await?;
    let counts = GateCounts {
        business_rules,
        requirements,
        acceptance_criteria,
        open_questions,
    };
    let mut missing = Vec::new();
    if business_rules == 0 {
        missing.push("Au moins une règle métier confirmée".into());
    }
    if requirements == 0 {
        missing.push("Au moins une exigence confirmée".into());
    }
    if acceptance_criteria == 0 {
        missing.push("Au moins un critère d’acceptation confirmé".into());
    }
    let warnings = if open_questions > 0 {
        vec![format!(
            "{open_questions} question(s) ouverte(s) restent à traiter"
        )]
    } else {
        Vec::new()
    };
    let status = if !missing.is_empty() {
        "blocked"
    } else if warnings.is_empty() {
        "passed"
    } else {
        "passed_with_warning"
    };
    let evaluation = GateResult {
        public_id: None,
        status: status.into(),
        graph_version: project.graph_version,
        missing,
        warnings,
        counts,
    };
    let node_id: i64 = sqlx::query_scalar(
        "select id from app.context_nodes where project_id = $1 and node_key = 'product'",
    )
    .bind(project.id)
    .fetch_one(&mut *tx)
    .await?;
    let public_id: Uuid = sqlx::query_scalar(
        "insert into app.gates (
           workspace_id, project_id, context_node_id, gate_key, status, evaluation, graph_version
         ) values ($1,$2,$3,'product-ready',$4,$5,$6)
         on conflict (project_id, gate_key, graph_version)
         do update set status = excluded.status, evaluation = excluded.evaluation, evaluated_at = now()
         returning public_id",
    )
    .bind(project.workspace_id)
    .bind(project.id)
    .bind(node_id)
    .bind(status)
    .bind(serde_json::to_value(&evaluation).map_err(|error| AppError::Internal(error.to_string()))?)
    .bind(project.graph_version)
    .fetch_one(&mut *tx)
    .await?;
    let result = GateResult {
        public_id: Some(public_id),
        ..evaluation
    };
    complete_command(&mut tx, lease, &result).await?;
    tx.commit().await?;
    Ok(result)
}

pub async fn generate_feature_brief(
    state: &AppState,
    project_public_id: Uuid,
) -> AppResult<DeliverableSummary> {
    generate_feature_brief_command(state, project_public_id, None).await
}

pub async fn generate_feature_brief_idempotent(
    state: &AppState,
    project_public_id: Uuid,
    lease: &IdempotencyLease,
) -> AppResult<DeliverableSummary> {
    generate_feature_brief_command(state, project_public_id, Some(lease)).await
}

async fn generate_feature_brief_command(
    state: &AppState,
    project_public_id: Uuid,
    lease: Option<&IdempotencyLease>,
) -> AppResult<DeliverableSummary> {
    let gate = evaluate_product_gate(state, project_public_id).await?;
    if gate.status == "blocked" {
        return Err(AppError::Invalid(format!(
            "ProductReadyGate is blocked: {}",
            gate.missing.join(", ")
        )));
    }
    let mut tx = state.begin_request().await?;
    let project = project_by_public(&mut tx, state.workspace_id, project_public_id).await?;
    let knowledge = knowledge_for_project(&mut tx, project.id)
        .await?
        .into_iter()
        .filter(|item| item.node_key == "product")
        .collect::<Vec<_>>();
    let content = json!({
        "objective": project.objective,
        "business_rules": knowledge.iter().filter(|item| item.entry_type == "business_rule").collect::<Vec<_>>(),
        "requirements": knowledge.iter().filter(|item| item.entry_type == "requirement").collect::<Vec<_>>(),
        "acceptance_criteria": knowledge.iter().filter(|item| item.entry_type == "acceptance_criterion").collect::<Vec<_>>(),
        "open_questions": knowledge.iter().filter(|item| item.entry_type == "open_question").collect::<Vec<_>>(),
        "source_version_ids": knowledge.iter().map(|item| item.version_public_id).collect::<Vec<_>>(),
    });
    let content_hash = sha256_json(&content)?;
    let previous: Option<(i64, Uuid, i32)> = sqlx::query_as(
        "select id, lineage_public_id, version from app.deliverables
         where project_id = $1 and deliverable_type = 'feature-brief'
           and status <> 'superseded'
         order by version desc, id desc limit 1 for update",
    )
    .bind(project.id)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some((previous_id, _, _)) = previous {
        sqlx::query("update app.deliverables set status = 'superseded' where id = $1")
            .bind(previous_id)
            .execute(&mut *tx)
            .await?;
    }
    let node_id: i64 = sqlx::query_scalar(
        "select id from app.context_nodes where project_id = $1 and node_key = 'product'",
    )
    .bind(project.id)
    .fetch_one(&mut *tx)
    .await?;
    let contract_id: i64 = sqlx::query_scalar(
        "select id from app.deliverable_contracts where template_id = $1 and contract_key = 'feature-brief'",
    )
    .bind(project.template_id)
    .fetch_one(&mut *tx)
    .await?;
    let (supersedes_deliverable_id, lineage_public_id, deliverable_version) = previous.map_or_else(
        || (None, Uuid::new_v4(), 1),
        |(id, lineage, version)| (Some(id), lineage, version + 1),
    );
    let row = sqlx::query_as::<_, DeliverableSummary>(
        "insert into app.deliverables (
           workspace_id, project_id, context_node_id, contract_id, lineage_public_id,
           supersedes_deliverable_id, source_graph_version, content_hash, deliverable_type,
           title, summary, content, status, coverage_status, version, committed_at
         ) values ($1,$2,$3,$4,$5,$6,$7,$8,'feature-brief',$9,$10,$11,
                   'committed','missing',$12,now())
         returning public_id, deliverable_type, title, summary, content, status,
                   coverage_status, version, updated_at",
    )
    .bind(project.workspace_id)
    .bind(project.id)
    .bind(node_id)
    .bind(contract_id)
    .bind(lineage_public_id)
    .bind(supersedes_deliverable_id)
    .bind(project.graph_version)
    .bind(&content_hash)
    .bind(format!("Feature Brief — {}", project.name))
    .bind("Cadrage Produit validé et prêt pour le handoff Tech.")
    .bind(&content)
    .bind(deliverable_version)
    .fetch_one(&mut *tx)
    .await?;
    let deliverable_id: i64 =
        sqlx::query_scalar("select id from app.deliverables where public_id = $1")
            .bind(row.public_id)
            .fetch_one(&mut *tx)
            .await?;
    let sections = [
        ("objective", "Objectif", project.objective.clone()),
        (
            "business_rules",
            "Règles métier",
            render_items(&knowledge, "business_rule"),
        ),
        (
            "requirements",
            "Exigences",
            render_items(&knowledge, "requirement"),
        ),
        (
            "acceptance_criteria",
            "Critères d’acceptation",
            render_items(&knowledge, "acceptance_criterion"),
        ),
        (
            "open_questions",
            "Questions ouvertes",
            render_items(&knowledge, "open_question"),
        ),
    ];
    for (ordinal, (key, title, body)) in (0_i32..).zip(sections) {
        sqlx::query(
            "insert into app.deliverable_sections (
               workspace_id, project_id, deliverable_id, section_key, title, body,
               ordinal, source_data
             ) values ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(project.workspace_id)
        .bind(project.id)
        .bind(deliverable_id)
        .bind(key)
        .bind(title)
        .bind(body)
        .bind(ordinal)
        .bind(json!({"source_version_ids": knowledge.iter().map(|item| item.version_public_id).collect::<Vec<_>>() }))
        .execute(&mut *tx)
        .await?;
    }
    for item in &knowledge {
        let version_id: i64 =
            sqlx::query_scalar("select id from app.knowledge_entry_versions where public_id = $1")
                .bind(item.version_public_id)
                .fetch_one(&mut *tx)
                .await?;
        sqlx::query(
            "insert into app.deliverable_sources (
               workspace_id, project_id, deliverable_id, source_kind, source_public_id,
               knowledge_entry_version_id, source_role, included_reason
             ) values ($1,$2,$3,'knowledge_entry_version',$4,$5,$6,
                       'Version confirmée incluse dans le Feature Brief')",
        )
        .bind(project.workspace_id)
        .bind(project.id)
        .bind(deliverable_id)
        .bind(item.version_public_id)
        .bind(version_id)
        .bind(&item.entry_type)
        .execute(&mut *tx)
        .await?;
    }
    audit(
        &mut tx,
        project.workspace_id,
        Some(project.id),
        state.actor_id,
        "deliverable.committed",
        "deliverable",
        row.public_id,
        None,
        Some(content),
    )
    .await?;
    complete_command(&mut tx, lease, &row).await?;
    tx.commit().await?;
    Ok(row)
}

pub async fn compile_context_pack(
    state: &AppState,
    project_public_id: Uuid,
    input: CompileContextPack,
) -> AppResult<ContextPackSummary> {
    compile_context_pack_command(state, project_public_id, input, None).await
}

pub async fn compile_context_pack_idempotent(
    state: &AppState,
    project_public_id: Uuid,
    input: CompileContextPack,
    lease: &IdempotencyLease,
) -> AppResult<ContextPackSummary> {
    compile_context_pack_command(state, project_public_id, input, Some(lease)).await
}

async fn compile_context_pack_command(
    state: &AppState,
    project_public_id: Uuid,
    input: CompileContextPack,
    lease: Option<&IdempotencyLease>,
) -> AppResult<ContextPackSummary> {
    if input.task_kind != "technical-delivery-plan" {
        return Err(AppError::Invalid(
            "only technical-delivery-plan ContextPacks are supported in the alpha".into(),
        ));
    }
    let gate = evaluate_product_gate(state, project_public_id).await?;
    if gate.status == "blocked" {
        return Err(AppError::Invalid(
            "ProductReadyGate must pass before handoff".into(),
        ));
    }
    let mut read_tx = state.begin_request().await?;
    let project = project_by_public(&mut read_tx, state.workspace_id, project_public_id).await?;
    if gate.graph_version != project.graph_version {
        return Err(AppError::Conflict(
            "the project context changed while evaluating ProductReadyGate".into(),
        ));
    }
    let source =
        session_by_public(&mut read_tx, state.workspace_id, input.source_session_id).await?;
    if source.project_id != project.id || source.node_key != "product" {
        return Err(AppError::Invalid(
            "handoff source must be a Product session from this project".into(),
        ));
    }
    let knowledge = knowledge_for_project(&mut read_tx, project.id).await?;
    let candidates = knowledge
        .iter()
        .map(|item| ContextCandidate {
            knowledge_public_id: item.public_id,
            version_public_id: item.version_public_id,
            version_number: item.version_number,
            entry_type: item.entry_type.clone(),
            title: item.title.clone(),
            statement: item.statement.clone(),
            rationale: item.rationale.clone(),
            node_key: item.node_key.clone(),
        })
        .collect::<Vec<_>>();
    let contract: Value = sqlx::query_scalar(
        "select jsonb_build_object(
           'contract_key', contract_key, 'name', name, 'required_sections', required_sections,
           'accepted_evidence_types', accepted_evidence_types, 'completion_rules', completion_rules
         ) from app.deliverable_contracts
         where template_id = $1 and contract_key = 'technical-delivery-plan'",
    )
    .bind(project.template_id)
    .fetch_one(&mut *read_tx)
    .await?;
    let selector_input = ContextSelectionInput {
        objective: project.objective.clone(),
        task_kind: input.task_kind.clone(),
        candidates: serde_json::to_value(&candidates)
            .map_err(|error| AppError::Internal(error.to_string()))?,
    };
    let input_hash = sha256_json(&selector_input)?;
    let candidate_source_ids = candidates
        .iter()
        .map(|candidate| candidate.version_public_id)
        .collect::<Vec<_>>();
    let model_run = start_model_run(
        &mut read_tx,
        state.engine.provider_name(),
        state.engine.requested_model(),
        ModelRunStart {
            workspace_id: project.workspace_id,
            project_id: project.id,
            session_id: None,
            context_pack_id: None,
            operation: "select_context",
            prompt_version: CONTEXT_SELECTION_PROMPT_VERSION,
            schema_version: CONTEXT_SELECTION_SCHEMA_VERSION,
            source_graph_version: project.graph_version,
            input_hash: &input_hash,
            source_public_ids: &candidate_source_ids,
        },
    )
    .await?;
    read_tx.commit().await?;

    // Provider work happens only after the durable running run was committed.
    let selection = match state.engine.select_context(selector_input).await {
        Ok(result) => result,
        Err(error) => {
            record_failed_model_run(state, model_run.id, &error).await?;
            return Err(error);
        }
    };
    let selection_output = match serde_json::to_value(&selection.output) {
        Ok(output) => output,
        Err(error) => {
            let error = AppError::Internal(error.to_string());
            record_failed_model_run_with_output(
                state,
                model_run.id,
                &selection.metadata,
                None,
                &error,
            )
            .await?;
            return Err(error);
        }
    };
    let compiled = match compile_context_projection(
        &project.objective,
        &project.summary,
        project.graph_version,
        &contract,
        &candidates,
        &selection.output.selected_version_ids,
        input.token_budget.unwrap_or(DEFAULT_CONTEXT_BUDGET_TOKENS),
        if state.agent_mode == "openai" {
            "hybrid"
        } else {
            "deterministic"
        },
    ) {
        Ok(compiled) => compiled,
        Err(error) => {
            record_failed_model_run_with_output(
                state,
                model_run.id,
                &selection.metadata,
                Some(&selection_output),
                &error,
            )
            .await?;
            return Err(error);
        }
    };

    let mut tx = state.begin_request().await?;
    let current_graph_version: i64 =
        sqlx::query_scalar("select graph_version from app.projects where id = $1 for update")
            .bind(project.id)
            .fetch_one(&mut *tx)
            .await?;
    if current_graph_version != project.graph_version {
        let error = AppError::Conflict(
            "the project context changed while compiling the ContextPack".into(),
        );
        fail_model_run_in_transaction(
            &mut tx,
            model_run.id,
            Some(&selection.metadata),
            Some(&selection_output),
            &error,
        )
        .await?;
        tx.commit().await?;
        return Err(error);
    }
    let product_node_id: i64 = sqlx::query_scalar(
        "select id from app.context_nodes where project_id = $1 and node_key = 'product'",
    )
    .bind(project.id)
    .fetch_one(&mut *tx)
    .await?;
    let (tech_node_id, tech_profile_id): (i64, i64) = sqlx::query_as(
        "select id, agent_profile_id from app.context_nodes where project_id = $1 and node_key = 'tech'",
    )
    .bind(project.id)
    .fetch_one(&mut *tx)
    .await?;
    let supersedes_context_pack_id: Option<i64> = sqlx::query_scalar(
        "select id from app.context_packs
         where project_id = $1 and target_node_id = $2 and task_kind = $3 and status = 'current'
         for update",
    )
    .bind(project.id)
    .bind(tech_node_id)
    .bind(&input.task_kind)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(previous_id) = supersedes_context_pack_id {
        sqlx::query(
            "update app.context_packs
             set status = 'superseded', invalidated_at = now(),
                 stale_reason = 'recompiled'
             where id = $1",
        )
        .bind(previous_id)
        .execute(&mut *tx)
        .await?;
    }
    let version: i32 = sqlx::query_scalar(
        "select coalesce(max(version), 0)::integer + 1 from app.context_packs
         where project_id = $1 and target_node_id = $2 and task_kind = $3",
    )
    .bind(project.id)
    .bind(tech_node_id)
    .bind(&input.task_kind)
    .fetch_one(&mut *tx)
    .await?;
    let (pack_id, pack_public_id): (i64, Uuid) = sqlx::query_as(
        "insert into app.context_packs (
           workspace_id, project_id, source_node_id, target_node_id, target_agent_profile_id,
           task_kind, objective, content, version, source_graph_version, compiler_version,
           selection_mode, content_hash, token_budget, estimated_tokens,
           supersedes_context_pack_id
         ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)
         returning id, public_id",
    )
    .bind(project.workspace_id)
    .bind(project.id)
    .bind(product_node_id)
    .bind(tech_node_id)
    .bind(tech_profile_id)
    .bind(&input.task_kind)
    .bind(&project.objective)
    .bind(&compiled.content)
    .bind(version)
    .bind(compiled.source_graph_version)
    .bind(&compiled.compiler_version)
    .bind(&compiled.selection_mode)
    .bind(&compiled.content_hash)
    .bind(compiled.token_budget)
    .bind(compiled.token_count)
    .bind(supersedes_context_pack_id)
    .fetch_one(&mut *tx)
    .await?;
    for selection_item in &compiled.selection_items {
        let (entry_id, version_id): (i64, i64) = sqlx::query_as(
            "select k.id, v.id from app.knowledge_entries k
             join app.knowledge_entry_versions v on v.knowledge_entry_id = k.id
             where k.public_id = $1 and v.public_id = $2",
        )
        .bind(selection_item.knowledge_public_id)
        .bind(selection_item.version_public_id)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            "insert into app.context_pack_selection_items (
               workspace_id, project_id, context_pack_id, candidate_kind,
               candidate_public_id, knowledge_entry_version_id, decision, reason_code,
               explanation, rank, estimated_tokens, is_mandatory
             ) values ($1,$2,$3,'knowledge_entry_version',$4,$5,$6,$7,$8,$9,$10,$11)",
        )
        .bind(project.workspace_id)
        .bind(project.id)
        .bind(pack_id)
        .bind(selection_item.version_public_id)
        .bind(version_id)
        .bind(if selection_item.included {
            "included"
        } else {
            "excluded"
        })
        .bind(&selection_item.reason_code)
        .bind(&selection_item.explanation)
        .bind(selection_item.rank)
        .bind(selection_item.token_estimate)
        .bind(selection_item.required)
        .execute(&mut *tx)
        .await?;
        if selection_item.included {
            let source_role = candidates
                .iter()
                .find(|candidate| candidate.version_public_id == selection_item.version_public_id)
                .map_or("knowledge", |candidate| candidate.entry_type.as_str());
            sqlx::query(
                "insert into app.context_pack_sources (
                   workspace_id, project_id, context_pack_id, knowledge_entry_id,
                   knowledge_entry_version_id, source_role, included_reason
                 ) values ($1,$2,$3,$4,$5,$6,$7)",
            )
            .bind(project.workspace_id)
            .bind(project.id)
            .bind(pack_id)
            .bind(entry_id)
            .bind(version_id)
            .bind(source_role)
            .bind(&selection_item.explanation)
            .execute(&mut *tx)
            .await?;
        }
    }
    complete_model_run(
        &mut tx,
        model_run.id,
        Some(pack_id),
        &selection.metadata,
        &selection_output,
    )
    .await?;
    audit(
        &mut tx,
        project.workspace_id,
        Some(project.id),
        state.actor_id,
        "context_pack.compiled",
        "context_pack",
        pack_public_id,
        None,
        Some(json!({
            "source_graph_version": compiled.source_graph_version,
            "content_hash": compiled.content_hash,
            "included_count": compiled.included_count,
            "candidate_count": compiled.candidate_count,
        })),
    )
    .await?;
    let pack = context_pack_by_id(&mut tx, pack_id).await?;
    complete_command(&mut tx, lease, &pack).await?;
    tx.commit().await?;
    Ok(pack)
}

pub async fn context_pack(
    state: &AppState,
    context_pack_public_id: Uuid,
) -> AppResult<ContextPackSummary> {
    let mut tx = state.begin_request().await?;
    let id: i64 = sqlx::query_scalar(
        "select cp.id from app.context_packs cp
         join app.workspaces w on w.id = cp.workspace_id
         where w.public_id = $1 and cp.public_id = $2",
    )
    .bind(state.workspace_id)
    .bind(context_pack_public_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    let pack = context_pack_by_id(&mut tx, id).await?;
    tx.commit().await?;
    Ok(pack)
}

pub async fn export_context_pack(
    state: &AppState,
    context_pack_public_id: Uuid,
    format: &str,
) -> AppResult<(&'static str, String)> {
    let pack = context_pack(state, context_pack_public_id).await?;
    match format {
        "json" => Ok((
            "application/json; charset=utf-8",
            serde_json::to_string_pretty(&pack)
                .map_err(|error| AppError::Internal(error.to_string()))?,
        )),
        "markdown" => Ok((
            "text/markdown; charset=utf-8",
            render_context_pack_markdown(&pack),
        )),
        _ => Err(AppError::Invalid(
            "context pack export format must be json or markdown".into(),
        )),
    }
}

pub async fn create_handoff(
    state: &AppState,
    project_public_id: Uuid,
    input: CreateHandoff,
) -> AppResult<HandoffView> {
    create_handoff_command(state, project_public_id, input, None).await
}

pub async fn create_handoff_idempotent(
    state: &AppState,
    project_public_id: Uuid,
    input: CreateHandoff,
    lease: &IdempotencyLease,
) -> AppResult<HandoffView> {
    create_handoff_command(state, project_public_id, input, Some(lease)).await
}

async fn create_handoff_command(
    state: &AppState,
    project_public_id: Uuid,
    input: CreateHandoff,
    lease: Option<&IdempotencyLease>,
) -> AppResult<HandoffView> {
    let mut tx = state.begin_request().await?;
    let project = project_by_public(&mut tx, state.workspace_id, project_public_id).await?;
    let source = session_by_public(&mut tx, state.workspace_id, input.source_session_id).await?;
    if source.project_id != project.id || source.node_key != "product" {
        return Err(AppError::Invalid(
            "handoff source must be a Product session from this project".into(),
        ));
    }
    let (pack_id, source_graph_version, pack_status, current_graph_version): (
        i64,
        i64,
        String,
        i64,
    ) = sqlx::query_as(
        "select pack.id, pack.source_graph_version, pack.status, project.graph_version
         from app.context_packs pack
         join app.projects project on project.id = pack.project_id
         where pack.public_id = $1 and pack.project_id = $2
         for update of project, pack",
    )
    .bind(input.context_pack_id)
    .bind(project.id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    if pack_status != "current" || source_graph_version != current_graph_version {
        return Err(AppError::Conflict(
            "ContextPack is stale; recompile before creating a handoff".into(),
        ));
    }
    if let Some(existing) = handoff_by_source_and_pack(&mut tx, source.id, pack_id).await? {
        complete_command(&mut tx, lease, &existing).await?;
        tx.commit().await?;
        return Ok(existing);
    }
    let (tech_node_id, tech_profile_id): (i64, i64) = sqlx::query_as(
        "select id, agent_profile_id from app.context_nodes
         where project_id = $1 and node_key = 'tech'",
    )
    .bind(project.id)
    .fetch_one(&mut *tx)
    .await?;
    let target_session_public_id: Uuid = sqlx::query_scalar(
        "insert into app.sessions (
           workspace_id, project_id, context_node_id, agent_profile_id, context_pack_id,
           title, created_by_actor_id
         ) values ($1,$2,$3,$4,$5,'Plan de livraison Tech',$6) returning public_id",
    )
    .bind(project.workspace_id)
    .bind(project.id)
    .bind(tech_node_id)
    .bind(tech_profile_id)
    .bind(pack_id)
    .bind(state.actor_id)
    .fetch_one(&mut *tx)
    .await?;
    let pack_public_id = input.context_pack_id;
    let public_id: Uuid = sqlx::query_scalar(
        "insert into app.handoffs (
           workspace_id, project_id, source_session_id, target_session_id, context_pack_id,
           status, initiated_by_actor_id, completed_at
         ) select $1,$2,$3,s.id,$4,'completed',$5,now()
           from app.sessions s where s.public_id = $6 returning public_id",
    )
    .bind(project.workspace_id)
    .bind(project.id)
    .bind(source.id)
    .bind(pack_id)
    .bind(state.actor_id)
    .bind(target_session_public_id)
    .fetch_one(&mut *tx)
    .await?;
    audit(
        &mut tx,
        project.workspace_id,
        Some(project.id),
        state.actor_id,
        "handoff.completed",
        "handoff",
        public_id,
        None,
        Some(json!({"context_pack_public_id": pack_public_id, "target_session_public_id": target_session_public_id})),
    )
    .await?;
    let context_pack = context_pack_by_id(&mut tx, pack_id).await?;
    let handoff = HandoffView {
        public_id,
        context_pack_public_id: pack_public_id,
        target_session_public_id,
        status: "completed".into(),
        context_pack,
    };
    complete_command(&mut tx, lease, &handoff).await?;
    tx.commit().await?;
    Ok(handoff)
}

pub async fn latest_handoff(
    state: &AppState,
    project_public_id: Uuid,
) -> AppResult<Option<HandoffView>> {
    let mut tx = state.begin_request().await?;
    let project = project_by_public(&mut tx, state.workspace_id, project_public_id).await?;
    let handoff = latest_handoff_by_project_id(&mut tx, project.id).await?;
    tx.commit().await?;
    Ok(handoff)
}

fn render_items(items: &[KnowledgeSummary], entry_type: &str) -> String {
    let body = items
        .iter()
        .filter(|item| item.entry_type == entry_type)
        .map(|item| format!("- {} — {}", item.title, item.statement))
        .collect::<Vec<_>>()
        .join("\n");
    if body.is_empty() {
        "Aucun élément confirmé.".into()
    } else {
        body
    }
}

fn render_lines(items: &[String]) -> String {
    if items.is_empty() {
        "Aucun élément déclaré.".into()
    } else {
        items
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn render_context_pack_markdown(pack: &ContextPackSummary) -> String {
    let mut output = format!(
        "# ContextPack {} v{}\n\n- Status: `{}`\n- Source graph: `{}`\n- Compiler: `{}`\n- Selection: `{}`\n- Tokens: `{}/{}`\n- SHA-256: `{}`\n\n## Context\n\n```json\n{}\n```\n\n## Selection ledger\n",
        pack.public_id,
        pack.version,
        pack.status,
        pack.source_graph_version,
        pack.compiler_version,
        pack.selection_mode,
        pack.token_count,
        pack.token_budget,
        pack.content_hash,
        serde_json::to_string_pretty(&pack.content).unwrap_or_else(|_| "{}".into()),
    );
    for item in &pack.selection_items {
        let _ = write!(
            output,
            "\n- `{}` — **{}** (`{}`): {}",
            item.candidate_public_id, item.decision, item.reason_code, item.explanation
        );
    }
    output.push('\n');
    output
}

#[derive(sqlx::FromRow)]
#[allow(clippy::struct_field_names)]
struct RequirementRecord {
    knowledge_id: i64,
    version_id: i64,
    version_public_id: Uuid,
}

pub async fn generate_technical_plan(
    state: &AppState,
    project_public_id: Uuid,
    input: GenerateTechnicalPlan,
) -> AppResult<DeliverableSummary> {
    generate_technical_plan_command(state, project_public_id, input, None).await
}

pub async fn generate_technical_plan_idempotent(
    state: &AppState,
    project_public_id: Uuid,
    input: GenerateTechnicalPlan,
    lease: &IdempotencyLease,
) -> AppResult<DeliverableSummary> {
    generate_technical_plan_command(state, project_public_id, input, Some(lease)).await
}

async fn generate_technical_plan_command(
    state: &AppState,
    project_public_id: Uuid,
    input: GenerateTechnicalPlan,
    lease: Option<&IdempotencyLease>,
) -> AppResult<DeliverableSummary> {
    let mut read_tx = state.begin_request().await?;
    let project = project_by_public(&mut read_tx, state.workspace_id, project_public_id).await?;
    let session = session_by_public(&mut read_tx, state.workspace_id, input.session_id).await?;
    if session.project_id != project.id || session.node_key != "tech" {
        return Err(AppError::Invalid(
            "a Tech handoff session with a ContextPack is required".into(),
        ));
    }
    let pack_id = session.context_pack_id.ok_or_else(|| {
        AppError::Invalid("a Tech handoff session with a ContextPack is required".into())
    })?;
    let context_pack = context_pack_by_id(&mut read_tx, pack_id).await?;
    if context_pack.status != "current"
        || context_pack.source_graph_version != project.graph_version
    {
        return Err(AppError::Conflict(
            "the Tech session ContextPack is stale; recompile through a new handoff".into(),
        ));
    }
    let requirements = sqlx::query_as::<_, RequirementRecord>(
        "select k.id as knowledge_id, v.id as version_id,
                v.public_id as version_public_id
         from app.context_pack_sources source
         join app.knowledge_entries k on k.id = source.knowledge_entry_id
         join app.knowledge_entry_versions v on v.id = source.knowledge_entry_version_id
         where source.context_pack_id = $1 and v.entry_type = 'requirement'
         order by k.id",
    )
    .bind(pack_id)
    .fetch_all(&mut *read_tx)
    .await?;
    if requirements.is_empty() {
        return Err(AppError::Invalid(
            "at least one confirmed requirement is required".into(),
        ));
    }
    let pack_source_ids: std::collections::HashSet<Uuid> = sqlx::query_scalar(
        "select v.public_id from app.context_pack_sources source
         join app.knowledge_entry_versions v on v.id = source.knowledge_entry_version_id
         where source.context_pack_id = $1",
    )
    .bind(pack_id)
    .fetch_all(&mut *read_tx)
    .await?
    .into_iter()
    .collect();
    let mut pack_source_public_ids = pack_source_ids.iter().copied().collect::<Vec<_>>();
    pack_source_public_ids.sort_unstable();
    let plan_input = TechnicalPlanInput {
        objective: project.objective.clone(),
        context_pack: context_pack.content.clone(),
    };
    let input_hash = sha256_json(&plan_input)?;
    let model_run = start_model_run(
        &mut read_tx,
        state.engine.provider_name(),
        state.engine.requested_model(),
        ModelRunStart {
            workspace_id: project.workspace_id,
            project_id: project.id,
            session_id: Some(session.id),
            context_pack_id: Some(pack_id),
            operation: "generate_technical_plan",
            prompt_version: TECHNICAL_PLAN_PROMPT_VERSION,
            schema_version: TECHNICAL_PLAN_SCHEMA_VERSION,
            source_graph_version: project.graph_version,
            input_hash: &input_hash,
            source_public_ids: &pack_source_public_ids,
        },
    )
    .await?;
    read_tx.commit().await?;

    let generated = match state.engine.generate_technical_plan(plan_input).await {
        Ok(result) => result,
        Err(error) => {
            record_failed_model_run(state, model_run.id, &error).await?;
            return Err(error);
        }
    };
    let generated_output = match serde_json::to_value(&generated.output) {
        Ok(output) => output,
        Err(error) => {
            let error = AppError::Internal(error.to_string());
            record_failed_model_run_with_output(
                state,
                model_run.id,
                &generated.metadata,
                None,
                &error,
            )
            .await?;
            return Err(error);
        }
    };
    let requirement_ids = requirements
        .iter()
        .map(|requirement| requirement.version_public_id)
        .collect::<std::collections::HashSet<_>>();
    let validation =
        validate_technical_plan_output(&generated.output, &pack_source_ids, &requirement_ids);
    if let Err(error) = validation {
        record_failed_model_run_with_output(
            state,
            model_run.id,
            &generated.metadata,
            Some(&generated_output),
            &error,
        )
        .await?;
        return Err(error);
    }
    let content = generated_output;
    let content_hash = sha256_json(&content)?;
    let mut tx = state.begin_request().await?;
    let (current_graph_version, current_pack_status): (i64, String) = sqlx::query_as(
        "select project.graph_version, pack.status
         from app.projects project
         join app.context_packs pack on pack.project_id = project.id
         where project.id = $1 and pack.id = $2
         for update of project, pack",
    )
    .bind(project.id)
    .bind(pack_id)
    .fetch_one(&mut *tx)
    .await?;
    if current_graph_version != project.graph_version || current_pack_status != "current" {
        let error = AppError::Conflict(
            "the ContextPack became stale while generating the technical plan".into(),
        );
        fail_model_run_in_transaction(
            &mut tx,
            model_run.id,
            Some(&generated.metadata),
            Some(&content),
            &error,
        )
        .await?;
        tx.commit().await?;
        return Err(error);
    }
    let previous: Option<(i64, Uuid, i32)> = sqlx::query_as(
        "select id, lineage_public_id, version from app.deliverables
         where project_id = $1 and deliverable_type = 'technical-delivery-plan'
           and status <> 'superseded'
         order by version desc, id desc limit 1 for update",
    )
    .bind(project.id)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some((previous_id, _, _)) = previous {
        sqlx::query("update app.deliverables set status = 'superseded' where id = $1")
            .bind(previous_id)
            .execute(&mut *tx)
            .await?;
    }
    let contract_id: i64 = sqlx::query_scalar(
        "select id from app.deliverable_contracts
         where template_id = $1 and contract_key = 'technical-delivery-plan'",
    )
    .bind(project.template_id)
    .fetch_one(&mut *tx)
    .await?;
    let (supersedes_deliverable_id, lineage_public_id, deliverable_version) = previous.map_or_else(
        || (None, Uuid::new_v4(), 1),
        |(id, lineage, version)| (Some(id), lineage, version + 1),
    );
    let deliverable = sqlx::query_as::<_, DeliverableSummary>(
        "insert into app.deliverables (
           workspace_id, project_id, context_node_id, contract_id, source_session_id,
           source_context_pack_id, lineage_public_id, supersedes_deliverable_id,
           source_graph_version, content_hash, deliverable_type, title, summary, content,
           status, coverage_status, version, committed_at
         ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'technical-delivery-plan',$11,$12,$13,
                   'committed','missing',$14,now())
         returning public_id, deliverable_type, title, summary, content, status,
                   coverage_status, version, updated_at",
    )
    .bind(project.workspace_id)
    .bind(project.id)
    .bind(session.context_node_id)
    .bind(contract_id)
    .bind(session.id)
    .bind(session.context_pack_id)
    .bind(lineage_public_id)
    .bind(supersedes_deliverable_id)
    .bind(project.graph_version)
    .bind(&content_hash)
    .bind(&generated.output.title)
    .bind(&generated.output.summary)
    .bind(&content)
    .bind(deliverable_version)
    .fetch_one(&mut *tx)
    .await?;
    let deliverable_id: i64 =
        sqlx::query_scalar("select id from app.deliverables where public_id = $1")
            .bind(deliverable.public_id)
            .fetch_one(&mut *tx)
            .await?;
    let mut sections = vec![(
        "architecture".to_owned(),
        "Architecture".to_owned(),
        generated.output.architecture.clone(),
        Vec::<Uuid>::new(),
    )];
    for (index, slice) in generated.output.delivery_slices.iter().enumerate() {
        sections.push((
            format!("delivery-slice-{}", index + 1),
            slice.title.clone(),
            slice.body.clone(),
            slice.source_version_ids.clone(),
        ));
    }
    sections.push((
        "risks".to_owned(),
        "Risques".to_owned(),
        render_lines(&generated.output.risks),
        Vec::new(),
    ));
    sections.push((
        "validation".to_owned(),
        "Validation attendue".to_owned(),
        render_lines(&generated.output.validation),
        Vec::new(),
    ));
    sections.push((
        "coverage".to_owned(),
        "Couverture".to_owned(),
        "Aucune exigence n'est couverte tant qu'une preuve externe n'a pas été validée.".to_owned(),
        requirements
            .iter()
            .map(|item| item.version_public_id)
            .collect(),
    ));
    let mut section_ids = Vec::new();
    for (ordinal, (key, title, body, source_version_ids)) in (0_i32..).zip(sections) {
        let (id, public_id): (i64, Uuid) = sqlx::query_as(
            "insert into app.deliverable_sections (
               workspace_id, project_id, deliverable_id, section_key, title, body,
               ordinal, source_data
             ) values ($1,$2,$3,$4,$5,$6,$7,$8) returning id, public_id",
        )
        .bind(project.workspace_id)
        .bind(project.id)
        .bind(deliverable_id)
        .bind(&key)
        .bind(&title)
        .bind(&body)
        .bind(ordinal)
        .bind(json!({"source_version_ids": source_version_ids}))
        .fetch_one(&mut *tx)
        .await?;
        section_ids.push((key, id, public_id));
    }
    let (_, coverage_section_id, _) = section_ids
        .iter()
        .find(|(key, _, _)| key == "coverage")
        .ok_or_else(|| AppError::Internal("coverage section missing".into()))?;
    for requirement in &requirements {
        sqlx::query(
            "insert into app.requirement_coverage (
               workspace_id, project_id, requirement_entry_id, requirement_version_id,
               deliverable_id, deliverable_section_id, status, explanation
             ) values ($1,$2,$3,$4,$5,$6,'missing',
               'Aucune preuve externe validée n’est encore attachée à cette exigence.')",
        )
        .bind(project.workspace_id)
        .bind(project.id)
        .bind(requirement.knowledge_id)
        .bind(requirement.version_id)
        .bind(deliverable_id)
        .bind(*coverage_section_id)
        .execute(&mut *tx)
        .await?;
    }
    complete_model_run(
        &mut tx,
        model_run.id,
        Some(pack_id),
        &generated.metadata,
        &content,
    )
    .await?;
    let model_run_id = model_run.id;
    let model_run_public_id = model_run.public_id;
    sqlx::query(
        "insert into app.deliverable_sources (
           workspace_id, project_id, deliverable_id, source_kind, source_public_id,
           context_pack_id, source_role, included_reason
         ) values ($1,$2,$3,'context_pack',$4,$5,'compiled_context',
                   'Technical plan consumed this immutable ContextPack')",
    )
    .bind(project.workspace_id)
    .bind(project.id)
    .bind(deliverable_id)
    .bind(context_pack.public_id)
    .bind(pack_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "insert into app.deliverable_sources (
           workspace_id, project_id, deliverable_id, source_kind, source_public_id,
           model_run_id, source_role, included_reason
         ) values ($1,$2,$3,'model_run',$4,$5,'generation_run',
                   'Structured model run that produced this plan')",
    )
    .bind(project.workspace_id)
    .bind(project.id)
    .bind(deliverable_id)
    .bind(model_run_public_id)
    .bind(model_run_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "insert into app.deliverable_sources (
           workspace_id, project_id, deliverable_id, source_kind, source_public_id,
           knowledge_entry_version_id, source_role, included_reason
         )
         select $1,$2,$3,'knowledge_entry_version',v.public_id,v.id,
                source.source_role,source.included_reason
         from app.context_pack_sources source
         join app.knowledge_entry_versions v on v.id = source.knowledge_entry_version_id
         where source.context_pack_id = $4",
    )
    .bind(project.workspace_id)
    .bind(project.id)
    .bind(deliverable_id)
    .bind(pack_id)
    .execute(&mut *tx)
    .await?;
    let gap_exists: bool = sqlx::query_scalar(
        "select exists(select 1 from app.insights
         where project_id = $1 and insight_type = 'coverage_gap' and status in ('candidate','open','accepted'))",
    )
    .bind(project.id)
    .fetch_one(&mut *tx)
    .await?;
    if !gap_exists {
        let (insight_id, _): (i64, Uuid) = sqlx::query_as(
            "insert into app.insights (
               workspace_id, project_id, insight_type, status, severity, confidence, title, explanation
             ) values ($1,$2,'coverage_gap','open','warning',0.950,
               'Preuve exécutable manquante',
               'Le plan couvre les exigences par analyse documentaire, mais aucun test exécutable n’est encore attaché.')
             returning id, public_id",
        )
        .bind(project.workspace_id)
        .bind(project.id)
        .fetch_one(&mut *tx)
        .await?;
        for requirement in &requirements {
            sqlx::query(
                "insert into app.insight_sources (
                   workspace_id, project_id, insight_id, source_role, object_kind,
                   object_public_id, knowledge_entry_version_id
                 ) values ($1,$2,$3,'unproven_requirement','knowledge_entry_version',$4,$5)",
            )
            .bind(project.workspace_id)
            .bind(project.id)
            .bind(insight_id)
            .bind(requirement.version_public_id)
            .bind(requirement.version_id)
            .execute(&mut *tx)
            .await?;
        }
    }
    audit(
        &mut tx,
        project.workspace_id,
        Some(project.id),
        state.actor_id,
        "deliverable.committed",
        "deliverable",
        deliverable.public_id,
        None,
        Some(content),
    )
    .await?;
    complete_command(&mut tx, lease, &deliverable).await?;
    tx.commit().await?;
    Ok(deliverable)
}

pub async fn coverage(state: &AppState, project_public_id: Uuid) -> AppResult<CoverageView> {
    let mut tx = state.begin_request().await?;
    let project = project_by_public(&mut tx, state.workspace_id, project_public_id).await?;
    let items = sqlx::query_as::<_, CoverageItem>(
        "select k.public_id as requirement_public_id,
                v.public_id as requirement_version_public_id,
                v.title as requirement_title, v.statement as requirement_statement,
                d.public_id as deliverable_public_id, ds.public_id as section_public_id,
                e.public_id as evidence_public_id, rc.status, rc.explanation
         from app.requirement_coverage rc
         join app.knowledge_entries k on k.id = rc.requirement_entry_id
         join app.knowledge_entry_versions v on v.id = rc.requirement_version_id
         join app.deliverables d on d.id = rc.deliverable_id
         left join app.deliverable_sections ds on ds.id = rc.deliverable_section_id
         left join app.evidences e on e.id = rc.evidence_id
         where rc.project_id = $1 and d.status <> 'superseded'
         order by v.id",
    )
    .bind(project.id)
    .fetch_all(&mut *tx)
    .await?;
    let view = CoverageView {
        total: items.len(),
        covered: items.iter().filter(|item| item.status == "covered").count(),
        partial: items.iter().filter(|item| item.status == "partial").count(),
        missing: items.iter().filter(|item| item.status == "missing").count(),
        items,
    };
    tx.commit().await?;
    Ok(view)
}

pub async fn list_insights(state: &AppState) -> AppResult<Vec<InsightSummary>> {
    let mut tx = state.begin_request().await?;
    let insights = sqlx::query_as::<_, InsightSummary>(
        "select i.public_id, p.public_id as project_public_id, p.name as project_name,
                p.graph_version as project_graph_version,
                i.insight_type, i.status, i.severity,
                i.confidence::double precision as confidence, i.title, i.explanation,
                i.resolution_justification, i.detected_at, i.updated_at
         from app.insights i
         join app.workspaces w on w.id = i.workspace_id
         join app.projects p on p.id = i.project_id
         where w.public_id = $1 order by
           case i.severity when 'blocking' then 0 when 'warning' then 1 else 2 end,
           i.detected_at desc, i.id desc",
    )
    .bind(state.workspace_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(insights)
}

pub async fn insight_detail(state: &AppState, public_id: Uuid) -> AppResult<InsightDetail> {
    let mut tx = state.begin_request().await?;
    let detail = insight_detail_in_transaction(&mut tx, state.workspace_id, public_id).await?;
    tx.commit().await?;
    Ok(detail)
}

async fn insight_detail_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    public_id: Uuid,
) -> AppResult<InsightDetail> {
    let insight = sqlx::query_as::<_, InsightSummary>(
        "select i.public_id, p.public_id as project_public_id, p.name as project_name,
                p.graph_version as project_graph_version,
                i.insight_type, i.status, i.severity,
                i.confidence::double precision as confidence, i.title, i.explanation,
                i.resolution_justification, i.detected_at, i.updated_at
         from app.insights i
         join app.workspaces w on w.id = i.workspace_id
         join app.projects p on p.id = i.project_id
         where w.public_id = $1 and i.public_id = $2",
    )
    .bind(workspace_id)
    .bind(public_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(AppError::NotFound)?;
    let sources = sqlx::query_as::<_, InsightSourceView>(
        "select s.source_role, s.object_kind, s.object_public_id,
                k.public_id as knowledge_public_id, v.public_id as version_public_id,
                v.title as version_title, v.statement as version_statement
         from app.insight_sources s
         join app.insights i on i.id = s.insight_id
         left join app.knowledge_entry_versions v on v.id = s.knowledge_entry_version_id
         left join app.knowledge_entries k on k.id = v.knowledge_entry_id
         where i.public_id = $1 order by s.id",
    )
    .bind(public_id)
    .fetch_all(&mut **tx)
    .await?;
    let detail = InsightDetail { insight, sources };
    Ok(detail)
}

pub async fn insight_detail_for_project(
    state: &AppState,
    project_public_id: Uuid,
    insight_public_id: Uuid,
) -> AppResult<InsightDetail> {
    let detail = insight_detail(state, insight_public_id).await?;
    if detail.insight.project_public_id != project_public_id {
        return Err(AppError::NotFound);
    }
    Ok(detail)
}

pub async fn act_on_insight(
    state: &AppState,
    public_id: Uuid,
    input: InsightAction,
) -> AppResult<InsightDetail> {
    act_on_insight_command(state, public_id, input, None).await
}

async fn act_on_insight_command(
    state: &AppState,
    public_id: Uuid,
    input: InsightAction,
    lease: Option<&IdempotencyLease>,
) -> AppResult<InsightDetail> {
    let justification = input.justification.trim();
    if justification.is_empty() {
        return Err(AppError::Invalid("a justification is required".into()));
    }
    let next_status = match input.action {
        InsightDecision::Accept => "accepted",
        InsightDecision::Dismiss => "dismissed",
        InsightDecision::Resolve => {
            return Err(AppError::Invalid(
                "resolve requires a context mutation through the dedicated resolve endpoint".into(),
            ));
        }
    };
    let mut tx = state.begin_request().await?;
    let row: Option<(i64, i64, i64, String)> = sqlx::query_as(
        "select i.id, i.workspace_id, i.project_id, i.status
         from app.insights i join app.workspaces w on w.id = i.workspace_id
         where w.public_id = $1 and i.public_id = $2 for update of i",
    )
    .bind(state.workspace_id)
    .bind(public_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (insight_id, workspace_id, project_id, before_status) = row.ok_or(AppError::NotFound)?;
    if matches!(before_status.as_str(), "resolved" | "dismissed") {
        return Err(AppError::Conflict("insight is already closed".into()));
    }
    let graph_version: i64 =
        sqlx::query_scalar("select graph_version from app.projects where id = $1")
            .bind(project_id)
            .fetch_one(&mut *tx)
            .await?;
    sqlx::query(
        "update app.insights set status = $2, resolution_justification = $3,
                resolved_by_actor_id = $4, resolved_at = now(), updated_at = now()
         where public_id = $1",
    )
    .bind(public_id)
    .bind(next_status)
    .bind(justification)
    .bind(state.actor_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "insert into app.insight_resolutions (
           workspace_id, project_id, insight_id, action, source_graph_version,
           resulting_graph_version, justification, resolved_by_actor_id
         ) values ($1,$2,$3,$4,$5,$5,$6,$7)",
    )
    .bind(workspace_id)
    .bind(project_id)
    .bind(insight_id)
    .bind(match input.action {
        InsightDecision::Accept => "accept",
        InsightDecision::Dismiss => "dismiss",
        InsightDecision::Resolve => unreachable!("resolve returned above"),
    })
    .bind(graph_version)
    .bind(justification)
    .bind(state.actor_id)
    .execute(&mut *tx)
    .await?;
    audit(
        &mut tx,
        workspace_id,
        Some(project_id),
        state.actor_id,
        "insight.status_changed",
        "insight",
        public_id,
        Some(json!({"status": before_status})),
        Some(json!({"status": next_status, "justification": justification})),
    )
    .await?;
    let result = insight_detail_in_transaction(&mut tx, state.workspace_id, public_id).await?;
    complete_command(&mut tx, lease, &result).await?;
    tx.commit().await?;
    Ok(result)
}

pub async fn act_on_insight_for_project(
    state: &AppState,
    project_public_id: Uuid,
    insight_public_id: Uuid,
    input: InsightAction,
) -> AppResult<InsightDetail> {
    insight_detail_for_project(state, project_public_id, insight_public_id).await?;
    act_on_insight_command(state, insight_public_id, input, None).await
}

pub async fn act_on_insight_for_project_idempotent(
    state: &AppState,
    project_public_id: Uuid,
    insight_public_id: Uuid,
    input: InsightAction,
    lease: &IdempotencyLease,
) -> AppResult<InsightDetail> {
    insight_detail_for_project(state, project_public_id, insight_public_id).await?;
    act_on_insight_command(state, insight_public_id, input, Some(lease)).await
}

pub async fn resolve_insight(
    state: &AppState,
    project_public_id: Uuid,
    insight_public_id: Uuid,
    input: ResolveInsight,
) -> AppResult<InsightDetail> {
    resolve_insight_command(state, project_public_id, insight_public_id, input, None).await
}

pub async fn resolve_insight_idempotent(
    state: &AppState,
    project_public_id: Uuid,
    insight_public_id: Uuid,
    input: ResolveInsight,
    lease: &IdempotencyLease,
) -> AppResult<InsightDetail> {
    resolve_insight_command(
        state,
        project_public_id,
        insight_public_id,
        input,
        Some(lease),
    )
    .await
}

async fn resolve_insight_command(
    state: &AppState,
    project_public_id: Uuid,
    insight_public_id: Uuid,
    input: ResolveInsight,
    lease: Option<&IdempotencyLease>,
) -> AppResult<InsightDetail> {
    let justification = input.justification.trim();
    if justification.is_empty() {
        return Err(AppError::Invalid("a justification is required".into()));
    }
    if input.mutations.len() != 1 {
        return Err(AppError::Invalid(
            "the alpha resolves an insight through exactly one source revision".into(),
        ));
    }
    let mut tx = state.begin_request().await?;
    let project = project_by_public(&mut tx, state.workspace_id, project_public_id).await?;
    let current_graph_version: i64 =
        sqlx::query_scalar("select graph_version from app.projects where id = $1 for update")
            .bind(project.id)
            .fetch_one(&mut *tx)
            .await?;
    if current_graph_version != input.expected_graph_version {
        return Err(AppError::Conflict(format!(
            "graph version changed from {} to {current_graph_version}",
            input.expected_graph_version
        )));
    }
    let (insight_id, insight_status): (i64, String) = sqlx::query_as(
        "select id, status from app.insights
         where project_id = $1 and public_id = $2 for update",
    )
    .bind(project.id)
    .bind(insight_public_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    if matches!(insight_status.as_str(), "resolved" | "dismissed") {
        return Err(AppError::Conflict("insight is already closed".into()));
    }

    let InsightResolutionMutation::ReviseKnowledge {
        knowledge_public_id,
        expected_version_public_id,
        statement,
        title,
        rationale,
    } = &input.mutations[0];
    let statement = statement.trim();
    if statement.is_empty() {
        return Err(AppError::Invalid("knowledge statement is required".into()));
    }
    let row: Option<ResolutionKnowledgeRow> = sqlx::query_as(
        "select k.id, k.workspace_id, k.context_node_id, k.latest_version,
                v.entry_type, v.title, v.rationale, v.id
         from app.knowledge_entries k
         join app.knowledge_entry_versions v
           on v.knowledge_entry_id = k.id and v.version_number = k.latest_version
         where k.project_id = $1 and k.public_id = $2 and v.public_id = $3
           and exists (
             select 1 from app.insight_sources source
             where source.insight_id = $4 and source.knowledge_entry_version_id = v.id
           )
         for update of k",
    )
    .bind(project.id)
    .bind(knowledge_public_id)
    .bind(expected_version_public_id)
    .bind(insight_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (
        knowledge_id,
        workspace_id,
        context_node_id,
        latest_version,
        entry_type,
        old_title,
        old_rationale,
        old_version_id,
    ) = row.ok_or_else(|| {
        AppError::Conflict(
            "the selected knowledge version is no longer a current source of this insight".into(),
        )
    })?;
    let next_version = latest_version + 1;
    let next_title = title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&old_title);
    let next_rationale = rationale
        .as_deref()
        .map_or(old_rationale.as_str(), str::trim);
    let (new_version_id, new_version_public_id): (i64, Uuid) = sqlx::query_as(
        "insert into app.knowledge_entry_versions (
           knowledge_entry_id, workspace_id, project_id, context_node_id, version_number,
           entry_type, title, statement, rationale, author_actor_id, origin_type,
           origin_public_id
         ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'insight_resolution',$11)
         returning id, public_id",
    )
    .bind(knowledge_id)
    .bind(workspace_id)
    .bind(project.id)
    .bind(context_node_id)
    .bind(next_version)
    .bind(entry_type)
    .bind(next_title)
    .bind(statement)
    .bind(next_rationale)
    .bind(state.actor_id)
    .bind(insight_public_id)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query("update app.knowledge_entries set latest_version = $2 where id = $1")
        .bind(knowledge_id)
        .bind(next_version)
        .execute(&mut *tx)
        .await?;
    let resulting_graph_version: i64 = sqlx::query_scalar(
        "update app.projects set graph_version = graph_version + 1
         where id = $1 returning graph_version",
    )
    .bind(project.id)
    .fetch_one(&mut *tx)
    .await?;
    invalidate_dependent_projections(&mut tx, project.id, &[old_version_id]).await?;
    sqlx::query(
        "update app.insights set status = 'resolved', resolution_justification = $2,
                resolved_by_actor_id = $3, resolved_at = now(), updated_at = now()
         where id = $1",
    )
    .bind(insight_id)
    .bind(justification)
    .bind(state.actor_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "insert into app.insight_resolutions (
           workspace_id, project_id, insight_id, action, knowledge_entry_version_id,
           source_graph_version, resulting_graph_version, justification,
           resolved_by_actor_id
         ) values ($1,$2,$3,'resolve',$4,$5,$6,$7,$8)",
    )
    .bind(workspace_id)
    .bind(project.id)
    .bind(insight_id)
    .bind(new_version_id)
    .bind(current_graph_version)
    .bind(resulting_graph_version)
    .bind(justification)
    .bind(state.actor_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "insert into app.domain_events (
           workspace_id, project_id, event_type, aggregate_kind, aggregate_public_id, payload
         ) values ($1,$2,'knowledge.revised','knowledge_entry',$3,$4)",
    )
    .bind(workspace_id)
    .bind(project.id)
    .bind(knowledge_public_id)
    .bind(json!({
        "source": "insight_resolution",
        "insight_public_id": insight_public_id,
        "version_public_id": new_version_public_id,
        "graph_version": resulting_graph_version,
    }))
    .execute(&mut *tx)
    .await?;
    audit(
        &mut tx,
        workspace_id,
        Some(project.id),
        state.actor_id,
        "insight.resolved_with_revision",
        "insight",
        insight_public_id,
        Some(json!({
            "status": insight_status,
            "graph_version": current_graph_version,
            "knowledge_version_public_id": expected_version_public_id,
        })),
        Some(json!({
            "status": "resolved",
            "graph_version": resulting_graph_version,
            "knowledge_version_public_id": new_version_public_id,
        })),
    )
    .await?;
    let result =
        insight_detail_in_transaction(&mut tx, state.workspace_id, insight_public_id).await?;
    complete_command(&mut tx, lease, &result).await?;
    tx.commit().await?;
    let _new_insight_ids = dispatch_steward_after_commit(state).await;
    Ok(result)
}

pub async fn project_history(
    state: &AppState,
    project_public_id: Uuid,
) -> AppResult<Vec<HistoryEvent>> {
    let mut tx = state.begin_request().await?;
    let project = project_by_public(&mut tx, state.workspace_id, project_public_id).await?;
    let history = sqlx::query_as::<_, HistoryEvent>(
        "select public_id, action, object_kind, object_public_id,
                before_state, after_state, occurred_at
         from app.audit_events where project_id = $1
         order by occurred_at desc, id desc limit 200",
    )
    .bind(project.id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(history)
}

type KnowledgeRevisionRow = (i64, i64, i64, i64, i32, String, String, String, i64);
type ResolutionKnowledgeRow = (i64, i64, i64, i32, String, String, String, i64);

pub async fn revise_knowledge_for_project(
    state: &AppState,
    project_public_id: Uuid,
    knowledge_public_id: Uuid,
    input: ReviseKnowledge,
) -> AppResult<RevisionResult> {
    revise_knowledge_command(state, project_public_id, knowledge_public_id, input, None).await
}

pub async fn revise_knowledge_for_project_idempotent(
    state: &AppState,
    project_public_id: Uuid,
    knowledge_public_id: Uuid,
    input: ReviseKnowledge,
    lease: &IdempotencyLease,
) -> AppResult<RevisionResult> {
    revise_knowledge_command(
        state,
        project_public_id,
        knowledge_public_id,
        input,
        Some(lease),
    )
    .await
}

async fn revise_knowledge_command(
    state: &AppState,
    project_public_id: Uuid,
    knowledge_public_id: Uuid,
    input: ReviseKnowledge,
    lease: Option<&IdempotencyLease>,
) -> AppResult<RevisionResult> {
    let statement = input.statement.trim();
    if statement.is_empty() {
        return Err(AppError::Invalid("knowledge statement is required".into()));
    }
    let mut tx = state.begin_request().await?;
    let row: Option<KnowledgeRevisionRow> = sqlx::query_as(
        "select k.id, k.workspace_id, k.project_id, k.context_node_id, k.latest_version,
                v.entry_type, v.title, v.rationale, v.id
         from app.knowledge_entries k
         join app.workspaces w on w.id = k.workspace_id
         join app.projects p on p.id = k.project_id
         join app.knowledge_entry_versions v
           on v.knowledge_entry_id = k.id and v.version_number = k.latest_version
         where w.public_id = $1 and p.public_id = $2 and k.public_id = $3
         for update of k",
    )
    .bind(state.workspace_id)
    .bind(project_public_id)
    .bind(knowledge_public_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (
        knowledge_id,
        workspace_id,
        project_id,
        node_id,
        latest_version,
        entry_type,
        old_title,
        old_rationale,
        old_version_id,
    ) = row.ok_or(AppError::NotFound)?;
    let next_version = latest_version + 1;
    let title = input
        .title
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or(&old_title);
    let rationale = input
        .rationale
        .as_deref()
        .map_or(old_rationale.as_str(), str::trim);
    let version_public_id: Uuid = sqlx::query_scalar(
        "insert into app.knowledge_entry_versions (
           knowledge_entry_id, workspace_id, project_id, context_node_id, version_number,
           entry_type, title, statement, rationale, author_actor_id, origin_type, origin_public_id
         ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'human_revision',$11)
         returning public_id",
    )
    .bind(knowledge_id)
    .bind(workspace_id)
    .bind(project_id)
    .bind(node_id)
    .bind(next_version)
    .bind(&entry_type)
    .bind(title)
    .bind(statement)
    .bind(rationale)
    .bind(state.actor_id)
    .bind(knowledge_public_id)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query("update app.knowledge_entries set latest_version = $2 where id = $1")
        .bind(knowledge_id)
        .bind(next_version)
        .execute(&mut *tx)
        .await?;
    let graph_version: i64 = sqlx::query_scalar(
        "update app.projects set graph_version = graph_version + 1 where id = $1 returning graph_version",
    )
    .bind(project_id)
    .fetch_one(&mut *tx)
    .await?;
    invalidate_dependent_projections(&mut tx, project_id, &[old_version_id]).await?;
    sqlx::query(
        "insert into app.domain_events (
           workspace_id, project_id, event_type, aggregate_kind, aggregate_public_id, payload
         ) values ($1,$2,'knowledge.revised','knowledge_entry',$3,$4)",
    )
    .bind(workspace_id)
    .bind(project_id)
    .bind(knowledge_public_id)
    .bind(json!({
        "version_public_id": version_public_id,
        "graph_version": graph_version,
    }))
    .execute(&mut *tx)
    .await?;
    audit(
        &mut tx,
        workspace_id,
        Some(project_id),
        state.actor_id,
        "knowledge.revised",
        "knowledge_entry",
        knowledge_public_id,
        Some(json!({"version_number": latest_version})),
        Some(json!({"version_number": next_version, "version_public_id": version_public_id, "statement": statement})),
    )
    .await?;
    let mut result = RevisionResult {
        knowledge_public_id,
        version_public_id,
        version_number: next_version,
        graph_version,
        insight_ids: Vec::new(),
    };
    complete_command(&mut tx, lease, &result).await?;
    tx.commit().await?;
    let insight_ids = dispatch_steward_after_commit(state).await;
    if lease.is_none() {
        result.insight_ids = insight_ids;
    }
    Ok(result)
}

async fn invalidate_projections(
    tx: &mut Transaction<'_, Postgres>,
    project_id: i64,
) -> AppResult<()> {
    sqlx::query(
        "update app.context_packs
                 set status = 'stale', invalidated_at = now(),
                     stale_reason = 'global invalidation fallback'
                 where project_id = $1 and status = 'current'",
    )
    .bind(project_id)
    .execute(&mut **tx)
    .await?;
    sqlx::query("update app.deliverables set status = 'stale', stale_at = now() where project_id = $1 and status = 'committed'")
        .bind(project_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn invalidate_dependent_projections(
    tx: &mut Transaction<'_, Postgres>,
    project_id: i64,
    _changed_version_ids: &[i64],
) -> AppResult<()> {
    sqlx::query(
        "update app.context_packs cp
         set status = 'stale', invalidated_at = now(),
             stale_reason = 'global invalidation fallback after graph revision'
         where cp.project_id = $1 and cp.status = 'current'
         ",
    )
    .bind(project_id)
    .execute(&mut **tx)
    .await?;
    let stale_deliverable_ids: Vec<i64> = sqlx::query_scalar(
        "update app.deliverables d
         set status = 'stale', stale_at = now()
         where d.project_id = $1 and d.status = 'committed'
         returning d.id",
    )
    .bind(project_id)
    .fetch_all(&mut **tx)
    .await?;
    if !stale_deliverable_ids.is_empty() {
        sqlx::query(
            "update app.evidences set status = 'stale'
             where deliverable_id = any($1) and status in ('candidate','valid')",
        )
        .bind(&stale_deliverable_ids)
        .execute(&mut **tx)
        .await?;
        sqlx::query(
            "update app.requirement_coverage
             set status = 'missing',
                 explanation = 'Projection obsolète après révision de contexte',
                 updated_at = now()
             where deliverable_id = any($1)",
        )
        .bind(&stale_deliverable_ids)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn dispatch_steward_after_commit(state: &AppState) -> Vec<Uuid> {
    if state.agent_mode == "deterministic" {
        return match crate::steward::drain_steward_outbox(state).await {
            Ok(summary) => summary.insight_public_ids,
            Err(error) => {
                tracing::warn!(
                    workspace_id = state.workspace_internal_id,
                    error_code = error.public_code(),
                    "Deterministic Steward outbox drain failed after committed context mutation"
                );
                Vec::new()
            }
        };
    }

    if let Some(trigger) = &state.steward_trigger {
        if let Err(error) = trigger.trigger_after_commit(state) {
            // The transaction is already committed. The durable pending
            // event is intentionally recovered by the bounded periodic scan
            // rather than converting this mutation into a client retry.
            tracing::warn!(
                workspace_id = state.workspace_internal_id,
                error_code = error.public_code(),
                "Steward trigger unavailable; pending work will recover on the next bounded scan"
            );
        }
    } else {
        tracing::warn!(
            workspace_id = state.workspace_internal_id,
            "OpenAI Steward supervisor is not configured; pending work awaits a configured recovery supervisor"
        );
    }
    Vec::new()
}

fn sha256_json<T: Serialize>(value: &T) -> AppResult<String> {
    let bytes = serde_json::to_vec(value).map_err(|error| AppError::Internal(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

async fn start_model_run(
    tx: &mut Transaction<'_, Postgres>,
    provider: &str,
    requested_model: &str,
    start: ModelRunStart<'_>,
) -> AppResult<ModelRunHandle> {
    let mut source_public_ids = start.source_public_ids.to_vec();
    source_public_ids.sort_unstable();
    source_public_ids.dedup();
    Ok(sqlx::query_as(
        "insert into app.model_runs (
           workspace_id, project_id, session_id, context_pack_id, operation,
           provider, model, prompt_version, schema_version, source_graph_version,
           input_hash, source_public_ids, status, attempt_count, started_at
         ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,
                   'running',0,clock_timestamp())
         returning id, public_id",
    )
    .bind(start.workspace_id)
    .bind(start.project_id)
    .bind(start.session_id)
    .bind(start.context_pack_id)
    .bind(start.operation)
    .bind(provider)
    .bind(requested_model)
    .bind(start.prompt_version)
    .bind(start.schema_version)
    .bind(start.source_graph_version)
    .bind(start.input_hash)
    .bind(source_public_ids)
    .fetch_one(&mut **tx)
    .await?)
}

async fn complete_model_run(
    tx: &mut Transaction<'_, Postgres>,
    model_run_id: i64,
    context_pack_id: Option<i64>,
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
    let updated = sqlx::query(
        "update app.model_runs
         set context_pack_id = coalesce($2, context_pack_id),
             provider = $3, model = $4, provider_response_id = $5,
             status = 'completed', output = $6, usage = $7,
             input_tokens = $8, output_tokens = $9,
             estimated_cost = $10::double precision::numeric,
             latency_ms = $11, attempt_count = $12,
             error_class = null, error_message = null,
             completed_at = clock_timestamp()
         where id = $1 and status = 'running'",
    )
    .bind(model_run_id)
    .bind(context_pack_id)
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
    if updated.rows_affected() != 1 {
        return Err(AppError::Conflict(
            "model run is no longer in the running state".into(),
        ));
    }
    Ok(())
}

async fn record_failed_model_run(
    state: &AppState,
    model_run_id: i64,
    error: &AppError,
) -> AppResult<()> {
    let mut tx = state.begin_request().await?;
    fail_model_run_in_transaction(&mut tx, model_run_id, None, None, error).await?;
    tx.commit().await?;
    Ok(())
}

async fn record_failed_model_run_with_output(
    state: &AppState,
    model_run_id: i64,
    metadata: &AgentRunMetadata,
    output: Option<&Value>,
    error: &AppError,
) -> AppResult<()> {
    let mut tx = state.begin_request().await?;
    fail_model_run_in_transaction(&mut tx, model_run_id, Some(metadata), output, error).await?;
    tx.commit().await?;
    Ok(())
}

async fn fail_model_run_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    model_run_id: i64,
    metadata: Option<&AgentRunMetadata>,
    output: Option<&Value>,
    error: &AppError,
) -> AppResult<()> {
    let updated = if let Some(metadata) = metadata {
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
                 status = 'failed', output = $5, usage = $6,
                 input_tokens = $7, output_tokens = $8,
                 estimated_cost = $9::double precision::numeric,
                 latency_ms = $10, attempt_count = $11,
                 error_class = $12, error_message = $13,
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
        .bind(error.model_run_error_class())
        .bind(bounded_model_run_error(&error.public_message()))
        .execute(&mut **tx)
        .await?
    } else {
        sqlx::query(
            "update app.model_runs
             set status = 'failed', attempt_count = greatest(attempt_count, $4),
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
                 error_class = $2, error_message = $3,
                 completed_at = clock_timestamp()
             where id = $1 and status = 'running'",
        )
        .bind(model_run_id)
        .bind(error.model_run_error_class())
        .bind(bounded_model_run_error(&error.public_message()))
        .bind(provider_attempt_count(error))
        .execute(&mut **tx)
        .await?
    };
    if updated.rows_affected() != 1 {
        return Err(AppError::Conflict(
            "model run is no longer in the running state".into(),
        ));
    }
    Ok(())
}

fn bounded_model_run_error(value: &str) -> String {
    value.chars().take(1_024).collect()
}

fn provider_attempt_count(error: &AppError) -> i32 {
    error
        .provider_attempts()
        .map_or(1, |attempts| i32::try_from(attempts).unwrap_or(i32::MAX))
}

fn validate_technical_plan_output(
    output: &TechnicalPlanDraft,
    pack_source_ids: &std::collections::HashSet<Uuid>,
    requirement_ids: &std::collections::HashSet<Uuid>,
) -> AppResult<()> {
    for source_id in output
        .delivery_slices
        .iter()
        .flat_map(|section| section.source_version_ids.iter())
    {
        if !pack_source_ids.contains(source_id) {
            return Err(AppError::Invalid(format!(
                "technical plan cited a source outside the ContextPack: {source_id}"
            )));
        }
    }
    if output
        .coverage
        .iter()
        .any(|item| !requirement_ids.contains(&item.requirement_version_public_id))
    {
        return Err(AppError::Invalid(
            "technical plan coverage cited an unknown requirement".into(),
        ));
    }
    Ok(())
}

async fn context_pack_by_id(
    connection: &mut PgConnection,
    id: i64,
) -> AppResult<ContextPackSummary> {
    let record = sqlx::query_as::<_, ContextPackRecord>(
        "select id, public_id, version, status, source_graph_version, compiler_version,
                selection_mode, content_hash, token_budget, estimated_tokens, compiled_at,
                invalidated_at, stale_reason, content
         from app.context_packs where id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or(AppError::NotFound)?;
    let selection_items = sqlx::query_as::<_, ContextPackSelectionItemView>(
        "select candidate_public_id, decision, reason_code, explanation, rank,
                estimated_tokens, is_mandatory
         from app.context_pack_selection_items
         where context_pack_id = $1
         order by decision desc, rank nulls last, id",
    )
    .bind(record.id)
    .fetch_all(&mut *connection)
    .await?;
    Ok(ContextPackSummary {
        public_id: record.public_id,
        version: record.version,
        status: record.status,
        source_graph_version: record.source_graph_version,
        compiler_version: record.compiler_version,
        selection_mode: record.selection_mode,
        content_hash: record.content_hash,
        token_budget: record.token_budget,
        token_count: record.estimated_tokens,
        compiled_at: record.compiled_at,
        invalidated_at: record.invalidated_at,
        stale_reason: record.stale_reason,
        content: record.content,
        selection_items,
    })
}

async fn handoff_view_from_row(
    connection: &mut PgConnection,
    row: (Uuid, i64, Uuid, Uuid, String),
) -> AppResult<HandoffView> {
    let (public_id, pack_id, context_pack_public_id, target_session_public_id, status) = row;
    Ok(HandoffView {
        public_id,
        context_pack_public_id,
        target_session_public_id,
        status,
        context_pack: context_pack_by_id(connection, pack_id).await?,
    })
}

async fn latest_handoff_by_project_id(
    connection: &mut PgConnection,
    project_id: i64,
) -> AppResult<Option<HandoffView>> {
    let row: Option<(Uuid, i64, Uuid, Uuid, String)> = sqlx::query_as(
        "select h.public_id, cp.id, cp.public_id, target.public_id, h.status
         from app.handoffs h
         join app.context_packs cp on cp.id = h.context_pack_id
         join app.sessions target on target.id = h.target_session_id
         where h.project_id = $1
         order by h.created_at desc, h.id desc limit 1",
    )
    .bind(project_id)
    .fetch_optional(&mut *connection)
    .await?;
    match row {
        Some(row) => Ok(Some(handoff_view_from_row(connection, row).await?)),
        None => Ok(None),
    }
}

async fn handoff_by_source_and_pack(
    connection: &mut PgConnection,
    source_session_id: i64,
    context_pack_id: i64,
) -> AppResult<Option<HandoffView>> {
    let row: Option<(Uuid, i64, Uuid, Uuid, String)> = sqlx::query_as(
        "select h.public_id, cp.id, cp.public_id, target.public_id, h.status
         from app.handoffs h
         join app.context_packs cp on cp.id = h.context_pack_id
         join app.sessions target on target.id = h.target_session_id
         where h.source_session_id = $1 and h.context_pack_id = $2
         order by h.id desc limit 1",
    )
    .bind(source_session_id)
    .bind(context_pack_id)
    .fetch_optional(&mut *connection)
    .await?;
    match row {
        Some(row) => Ok(Some(handoff_view_from_row(connection, row).await?)),
        None => Ok(None),
    }
}

async fn project_public_id_for_internal(
    connection: &mut PgConnection,
    project_id: i64,
) -> AppResult<Uuid> {
    sqlx::query_scalar("select public_id from app.projects where id = $1")
        .bind(project_id)
        .fetch_optional(&mut *connection)
        .await?
        .ok_or(AppError::NotFound)
}

async fn ensure_session_project(
    state: &AppState,
    project_public_id: Uuid,
    session_public_id: Uuid,
) -> AppResult<()> {
    let mut tx = state.begin_request().await?;
    let project = project_by_public(&mut tx, state.workspace_id, project_public_id).await?;
    let session = session_by_public(&mut tx, state.workspace_id, session_public_id).await?;
    if session.project_id != project.id {
        return Err(AppError::NotFound);
    }
    tx.commit().await?;
    Ok(())
}

async fn project_by_public(
    connection: &mut PgConnection,
    workspace_public_id: Uuid,
    public_id: Uuid,
) -> AppResult<ProjectRecord> {
    sqlx::query_as::<_, ProjectRecord>(
        "select p.id, p.workspace_id, p.template_id, p.public_id, p.name, p.objective, p.summary,
                p.status, p.graph_version, p.created_at, p.updated_at
         from app.projects p join app.workspaces w on w.id = p.workspace_id
         where w.public_id = $1 and p.public_id = $2",
    )
    .bind(workspace_public_id)
    .bind(public_id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or(AppError::NotFound)
}

async fn session_by_public(
    connection: &mut PgConnection,
    workspace_public_id: Uuid,
    public_id: Uuid,
) -> AppResult<SessionRecord> {
    sqlx::query_as::<_, SessionRecord>(
        "select s.id, s.workspace_id, s.project_id, s.context_node_id, s.agent_profile_id,
                s.context_pack_id, s.public_id, s.title, s.status, n.node_key,
                ap.scope_kind, ap.instructions, s.created_at, s.updated_at
         from app.sessions s
         join app.workspaces w on w.id = s.workspace_id
         join app.context_nodes n on n.id = s.context_node_id
         join app.agent_profiles ap on ap.id = s.agent_profile_id
         where w.public_id = $1 and s.public_id = $2",
    )
    .bind(workspace_public_id)
    .bind(public_id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or(AppError::NotFound)
}

async fn knowledge_for_project(
    connection: &mut PgConnection,
    project_id: i64,
) -> AppResult<Vec<KnowledgeSummary>> {
    Ok(sqlx::query_as::<_, KnowledgeSummary>(
        "select k.public_id, v.public_id as version_public_id, v.version_number, v.entry_type,
                v.title, v.statement, v.rationale, n.node_key, v.created_at
         from app.knowledge_entries k
         join app.knowledge_entry_versions v
           on v.knowledge_entry_id = k.id and v.version_number = k.latest_version
         join app.context_nodes n on n.id = v.context_node_id
         where k.project_id = $1 and k.status = 'confirmed' and v.status = 'confirmed'
         order by v.created_at, v.id",
    )
    .bind(project_id)
    .fetch_all(&mut *connection)
    .await?)
}

async fn insights_for_project(
    connection: &mut PgConnection,
    project_id: i64,
) -> AppResult<Vec<InsightSummary>> {
    Ok(sqlx::query_as::<_, InsightSummary>(
        "select i.public_id, p.public_id as project_public_id, p.name as project_name,
                p.graph_version as project_graph_version, i.insight_type, i.status,
                i.severity, i.confidence::double precision as confidence, i.title,
                i.explanation, i.resolution_justification, i.detected_at, i.updated_at
         from app.insights i join app.projects p on p.id = i.project_id
         where i.project_id = $1 order by i.detected_at desc, i.id desc",
    )
    .bind(project_id)
    .fetch_all(&mut *connection)
    .await?)
}

async fn latest_gate(
    connection: &mut PgConnection,
    project_id: i64,
) -> AppResult<Option<GateResult>> {
    let row: Option<(Uuid, String, i64, Value)> = sqlx::query_as(
        "select public_id, status, graph_version, evaluation
         from app.gates where project_id = $1 and gate_key = 'product-ready'
         order by graph_version desc, id desc limit 1",
    )
    .bind(project_id)
    .fetch_optional(&mut *connection)
    .await?;
    row.map(|(public_id, status, graph_version, evaluation)| {
        let mut result: GateResult = serde_json::from_value(evaluation)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        result.public_id = Some(public_id);
        result.status = status;
        result.graph_version = graph_version;
        Ok(result)
    })
    .transpose()
}

#[allow(clippy::too_many_arguments)]
async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: i64,
    project_id: Option<i64>,
    actor_id: Uuid,
    action: &str,
    object_kind: &str,
    object_public_id: Uuid,
    before_state: Option<Value>,
    after_state: Option<Value>,
) -> AppResult<()> {
    sqlx::query(
        "insert into app.audit_events (
           workspace_id, project_id, actor_id, action, object_kind, object_public_id,
           before_state, after_state
         ) values ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(workspace_id)
    .bind(project_id)
    .bind(actor_id)
    .bind(action)
    .bind(object_kind)
    .bind(object_public_id)
    .bind(before_state)
    .bind(after_state)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sqlx::postgres::PgPoolOptions;

    use super::AppState;
    use crate::{agent::DeterministicEngine, auth::RequestContext, error::AppError};

    fn unscoped_state() -> AppState {
        AppState {
            pool: PgPoolOptions::new()
                .connect_lazy("postgresql://postgres:postgres@127.0.0.1/postgres")
                .expect("test database URL is valid"),
            engine: Arc::new(DeterministicEngine),
            workspace_id: uuid::Uuid::nil(),
            workspace_internal_id: None,
            workspace_role: "viewer".into(),
            actor_id: uuid::Uuid::nil(),
            agent_mode: "deterministic",
            steward_trigger: None,
        }
    }

    #[tokio::test]
    async fn scoped_state_carries_the_authorized_internal_workspace_and_role() {
        let state = unscoped_state();
        let context = RequestContext {
            actor_id: uuid::Uuid::new_v4(),
            workspace_id: uuid::Uuid::new_v4(),
            workspace_internal_id: Some(42),
            workspace_role: "editor".into(),
        };

        let scoped = state.scoped(&context);

        assert_eq!(scoped.actor_id, context.actor_id);
        assert_eq!(scoped.workspace_id, context.workspace_id);
        assert_eq!(scoped.workspace_internal_id, Some(42));
        assert_eq!(scoped.workspace_role, "editor");
    }

    #[tokio::test]
    async fn business_transaction_refuses_an_unscoped_workspace() {
        let state = unscoped_state();
        let result = state.begin_request().await;
        assert!(matches!(
            result,
            Err(AppError::Internal(message))
                if message == "request-scoped workspace context is missing"
        ));
    }

    #[test]
    fn provider_attempts_are_typed_without_parsing_provider_payloads() {
        let exhausted = AppError::Provider(crate::error::ProviderError::new(
            "OpenAI",
            crate::error::ProviderErrorClass::Timeout,
            3,
            None,
        ));
        let unclassified = AppError::Agent("simulated provider unavailable".into());

        assert_eq!(super::provider_attempt_count(&exhausted), 3);
        assert_eq!(super::provider_attempt_count(&unclassified), 1);
        assert_eq!(exhausted.model_run_error_class(), "provider_timeout");
        assert_eq!(
            super::bounded_model_run_error(&"x".repeat(2_000))
                .chars()
                .count(),
            1_024
        );
    }

    #[test]
    fn provider_sources_must_be_an_exact_subset_of_the_authorized_context() {
        let allowed = [uuid::Uuid::new_v4(), uuid::Uuid::new_v4()];
        assert!(super::ensure_authorized_sources(&allowed[..1], &allowed).is_ok());
        assert!(super::ensure_authorized_sources(&[], &allowed).is_ok());
        assert!(matches!(
            super::ensure_authorized_sources(&[uuid::Uuid::new_v4()], &allowed),
            Err(AppError::Agent(message))
                if message.contains("outside the authorized project context")
        ));
    }
}
