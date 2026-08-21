#![allow(clippy::missing_errors_doc, clippy::too_many_lines)]

use std::sync::Arc;

use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    agent::{AgentEngine, AgentInput},
    error::{AppError, AppResult},
    models::{
        CommitResult, ContextNode, CoverageItem, CoverageView, CreateHandoff, CreateProject,
        CreateSession, DecideProposals, DeliverableSummary, GateCounts, GateResult,
        GenerateTechnicalPlan, HandoffView, Health, HistoryEvent, InsightAction, InsightDecision,
        InsightDetail, InsightSourceView, InsightSummary, KnowledgeSummary, MessageView,
        ProjectSnapshot, ProjectSummary, ProposalDecision, ProposalView, ReviseKnowledge,
        RevisionResult, SendMessage, SessionSummary, SessionView,
    },
};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub engine: Arc<dyn AgentEngine>,
    pub workspace_id: Uuid,
    pub actor_id: Uuid,
    pub agent_mode: &'static str,
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
    Ok(sqlx::query_as::<_, ProjectSummary>(
        "select p.public_id, p.name, p.objective, p.summary, p.status, p.graph_version, p.created_at, p.updated_at
         from app.projects p join app.workspaces w on w.id = p.workspace_id
         where w.public_id = $1 and p.status = 'active'
         order by p.updated_at desc, p.id desc",
    )
    .bind(state.workspace_id)
    .fetch_all(&state.pool)
    .await?)
}

pub async fn create_project(state: &AppState, input: CreateProject) -> AppResult<ProjectSummary> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::Invalid("project name is required".into()));
    }

    let mut tx = state.pool.begin().await?;
    let (workspace_id,): (i64,) =
        sqlx::query_as("select id from app.workspaces where public_id = $1 for share")
            .bind(state.workspace_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(AppError::NotFound)?;
    let (template_id,): (i64,) = sqlx::query_as(
        "select id from app.project_templates where template_key = 'software-product-delivery' for share",
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
    tx.commit().await?;
    Ok(project.summary())
}

pub async fn snapshot(state: &AppState, project_id: Uuid) -> AppResult<ProjectSnapshot> {
    let project = project_by_public(&state.pool, state.workspace_id, project_id).await?;
    let nodes = sqlx::query_as::<_, ContextNode>(
        "select n.public_id, n.node_key, n.title, n.description, n.summary,
                ap.profile_key, ap.name as profile_name, ap.scope_kind
         from app.context_nodes n join app.agent_profiles ap on ap.id = n.agent_profile_id
         where n.project_id = $1 order by n.id",
    )
    .bind(project.id)
    .fetch_all(&state.pool)
    .await?;
    let sessions = sqlx::query_as::<_, SessionSummary>(
        "select s.public_id, s.title, s.status, n.node_key, ap.scope_kind, s.created_at, s.updated_at
         from app.sessions s
         join app.context_nodes n on n.id = s.context_node_id
         join app.agent_profiles ap on ap.id = s.agent_profile_id
         where s.project_id = $1 order by s.updated_at desc, s.id desc",
    )
    .bind(project.id)
    .fetch_all(&state.pool)
    .await?;
    let knowledge = knowledge_for_project(&state.pool, project.id).await?;
    let deliverables = sqlx::query_as::<_, DeliverableSummary>(
        "select public_id, deliverable_type, title, summary, content, status, coverage_status, version, updated_at
         from app.deliverables where project_id = $1 order by updated_at desc, id desc",
    )
    .bind(project.id)
    .fetch_all(&state.pool)
    .await?;
    let insights = insights_for_project(&state.pool, project.id).await?;
    let gate = latest_gate(&state.pool, project.id).await?;
    Ok(ProjectSnapshot {
        project: project.summary(),
        nodes,
        sessions,
        knowledge,
        deliverables,
        insights,
        gate,
    })
}

pub async fn create_session(
    state: &AppState,
    project_public_id: Uuid,
    input: CreateSession,
) -> AppResult<SessionView> {
    let project = project_by_public(&state.pool, state.workspace_id, project_public_id).await?;
    let (node_id, agent_profile_id, default_title): (i64, i64, String) = sqlx::query_as(
        "select n.id, n.agent_profile_id, n.title
         from app.context_nodes n where n.project_id = $1 and n.node_key = $2",
    )
    .bind(project.id)
    .bind(&input.node_key)
    .fetch_optional(&state.pool)
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
    .fetch_one(&state.pool)
    .await?;
    session_view(state, public_id).await
}

pub async fn session_view(state: &AppState, public_id: Uuid) -> AppResult<SessionView> {
    let session = session_by_public(&state.pool, state.workspace_id, public_id).await?;
    let messages = sqlx::query_as::<_, MessageView>(
        "select public_id, role, content, agent_scope, metadata, created_at
         from app.messages where session_id = $1 order by created_at, id",
    )
    .bind(session.id)
    .fetch_all(&state.pool)
    .await?;
    let proposals = sqlx::query_as::<_, ProposalView>(
        "select public_id, entry_type, title, statement, rationale, status, source_data, created_at
         from app.mutation_proposals where session_id = $1 order by created_at, id",
    )
    .bind(session.id)
    .fetch_all(&state.pool)
    .await?;
    let context_pack = match session.context_pack_id {
        Some(id) => {
            sqlx::query_scalar::<_, Value>("select content from app.context_packs where id = $1")
                .bind(id)
                .fetch_optional(&state.pool)
                .await?
        }
        None => None,
    };
    Ok(SessionView {
        session: SessionSummary {
            public_id: session.public_id,
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
    })
}

pub async fn send_message(
    state: &AppState,
    session_id: Uuid,
    input: SendMessage,
) -> AppResult<SessionView> {
    let content = input.content.trim();
    if content.is_empty() {
        return Err(AppError::Invalid("message content is required".into()));
    }
    let session = session_by_public(&state.pool, state.workspace_id, session_id).await?;

    // Persist the user's intent before contacting an external provider.
    sqlx::query(
        "insert into app.messages (workspace_id, project_id, session_id, role, content)
         values ($1, $2, $3, 'user', $4)",
    )
    .bind(session.workspace_id)
    .bind(session.project_id)
    .bind(session.id)
    .bind(content)
    .execute(&state.pool)
    .await?;

    let knowledge = knowledge_for_project(&state.pool, session.project_id).await?;
    let knowledge_context = json!({
        "scope": session.scope_kind,
        "node": session.node_key,
        "context_pack": session.context_pack_id,
        "knowledge": knowledge,
    });
    let turn = state
        .engine
        .respond(AgentInput {
            scope_kind: session.scope_kind.clone(),
            instructions: session.instructions,
            user_message: content.into(),
            context: knowledge_context,
        })
        .await?;

    let mut tx = state.pool.begin().await?;
    let assistant_id: i64 = sqlx::query_scalar(
        "insert into app.messages (workspace_id, project_id, session_id, role, content, agent_scope, metadata)
         values ($1, $2, $3, 'assistant', $4, $5, $6) returning id",
    )
    .bind(session.workspace_id)
    .bind(session.project_id)
    .bind(session.id)
    .bind(turn.response)
    .bind(&session.scope_kind)
    .bind(json!({"sources": turn.sources}))
    .fetch_one(&mut *tx)
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
    tx.commit().await?;
    session_view(state, session_id).await
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
    if input.proposal_ids.is_empty() {
        return Err(AppError::Invalid(
            "at least one proposal is required".into(),
        ));
    }
    let session = session_by_public(&state.pool, state.workspace_id, session_public_id).await?;
    let mut tx = state.pool.begin().await?;
    let proposals = sqlx::query_as::<_, ProposalRecord>(
        "select mp.id, mp.public_id, m.public_id as source_message_public_id, mp.entry_type,
                mp.title, mp.statement, mp.rationale, mp.status, mp.source_data
         from app.mutation_proposals mp join app.messages m on m.id = mp.source_message_id
         where mp.session_id = $1 and mp.public_id = any($2) for update",
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
        tx.commit().await?;
        return Ok(CommitResult {
            confirmed,
            rejected: if desired_status == "rejected" {
                input.proposal_ids
            } else {
                Vec::new()
            },
            graph_version,
            insight_ids: Vec::new(),
        });
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
                    sqlx::query(
                        "insert into app.edges (
                           workspace_id, project_id, source_kind, source_public_id,
                           target_kind, target_public_id, edge_type, status, provenance,
                           created_by_actor_id
                         ) values ($1,$2,'knowledge_entry_version',$3,
                           'knowledge_entry_version',$4,'derived_from','confirmed',$5,$6)
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

    let insight_ids = if confirmed.is_empty() {
        Vec::new()
    } else {
        invalidate_projections(&mut tx, session.project_id).await?;
        detect_reference_conflict(&mut tx, session.workspace_id, session.project_id).await?
    };
    tx.commit().await?;
    Ok(CommitResult {
        confirmed,
        rejected,
        graph_version,
        insight_ids,
    })
}

pub async fn evaluate_product_gate(
    state: &AppState,
    project_public_id: Uuid,
) -> AppResult<GateResult> {
    let project = project_by_public(&state.pool, state.workspace_id, project_public_id).await?;
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
        .fetch_one(&state.pool)
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
    .fetch_one(&state.pool)
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
    .fetch_one(&state.pool)
    .await?;
    Ok(GateResult {
        public_id: Some(public_id),
        ..evaluation
    })
}

pub async fn generate_feature_brief(
    state: &AppState,
    project_public_id: Uuid,
) -> AppResult<DeliverableSummary> {
    let gate = evaluate_product_gate(state, project_public_id).await?;
    if gate.status == "blocked" {
        return Err(AppError::Invalid(format!(
            "ProductReadyGate is blocked: {}",
            gate.missing.join(", ")
        )));
    }
    let project = project_by_public(&state.pool, state.workspace_id, project_public_id).await?;
    let knowledge = knowledge_for_project(&state.pool, project.id)
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
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "update app.deliverables set status = 'superseded'
         where project_id = $1 and deliverable_type = 'feature-brief' and status <> 'superseded'",
    )
    .bind(project.id)
    .execute(&mut *tx)
    .await?;
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
    let row = sqlx::query_as::<_, DeliverableSummary>(
        "insert into app.deliverables (
           workspace_id, project_id, context_node_id, contract_id, deliverable_type,
           title, summary, content, status, coverage_status, committed_at
         ) values ($1,$2,$3,$4,'feature-brief',$5,$6,$7,'committed','covered',now())
         returning public_id, deliverable_type, title, summary, content, status,
                   coverage_status, version, updated_at",
    )
    .bind(project.workspace_id)
    .bind(project.id)
    .bind(node_id)
    .bind(contract_id)
    .bind(format!("Feature Brief — {}", project.name))
    .bind("Cadrage Produit validé et prêt pour le handoff Tech.")
    .bind(&content)
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
               deliverable_id, section_key, title, body, ordinal, source_data
             ) values ($1,$2,$3,$4,$5,$6)",
        )
        .bind(deliverable_id)
        .bind(key)
        .bind(title)
        .bind(body)
        .bind(ordinal)
        .bind(json!({"source_version_ids": knowledge.iter().map(|item| item.version_public_id).collect::<Vec<_>>() }))
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
    tx.commit().await?;
    Ok(row)
}

pub async fn create_handoff(
    state: &AppState,
    project_public_id: Uuid,
    input: CreateHandoff,
) -> AppResult<HandoffView> {
    let project = project_by_public(&state.pool, state.workspace_id, project_public_id).await?;
    let source =
        session_by_public(&state.pool, state.workspace_id, input.source_session_id).await?;
    if source.project_id != project.id || source.node_key != "product" {
        return Err(AppError::Invalid(
            "handoff source must be a Product session from this project".into(),
        ));
    }
    let gate = evaluate_product_gate(state, project_public_id).await?;
    if gate.status == "blocked" {
        return Err(AppError::Invalid(
            "ProductReadyGate must pass before handoff".into(),
        ));
    }
    let feature_brief_exists: bool = sqlx::query_scalar(
        "select exists(select 1 from app.deliverables
         where project_id = $1 and deliverable_type = 'feature-brief' and status = 'committed')",
    )
    .bind(project.id)
    .fetch_one(&state.pool)
    .await?;
    if !feature_brief_exists {
        generate_feature_brief(state, project_public_id).await?;
    }

    let mut tx = state.pool.begin().await?;
    let knowledge = knowledge_for_project(&state.pool, project.id).await?;
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
    let contract: Value = sqlx::query_scalar(
        "select jsonb_build_object(
           'contract_key', contract_key, 'name', name, 'required_sections', required_sections,
           'accepted_evidence_types', accepted_evidence_types, 'completion_rules', completion_rules
         ) from app.deliverable_contracts
         where template_id = $1 and contract_key = 'technical-delivery-plan'",
    )
    .bind(project.template_id)
    .fetch_one(&mut *tx)
    .await?;
    let content = json!({
        "objective": project.objective,
        "project_summary": project.summary,
        "graph_version": project.graph_version,
        "decisions": knowledge.iter().filter(|item| item.entry_type == "decision" || item.entry_type == "business_rule").collect::<Vec<_>>(),
        "requirements": knowledge.iter().filter(|item| item.entry_type == "requirement").collect::<Vec<_>>(),
        "acceptance_criteria": knowledge.iter().filter(|item| item.entry_type == "acceptance_criterion").collect::<Vec<_>>(),
        "constraints": knowledge.iter().filter(|item| item.entry_type == "constraint").collect::<Vec<_>>(),
        "open_questions": knowledge.iter().filter(|item| item.entry_type == "open_question").collect::<Vec<_>>(),
        "contract": contract,
        "provenance": knowledge.iter().map(|item| json!({
            "knowledge_public_id": item.public_id,
            "version_public_id": item.version_public_id,
            "version_number": item.version_number,
            "node": item.node_key,
        })).collect::<Vec<_>>(),
    });
    let version: i32 = sqlx::query_scalar(
        "select coalesce(max(version), 0)::integer + 1 from app.context_packs where project_id = $1 and task_kind = 'technical-delivery-plan'",
    )
    .bind(project.id)
    .fetch_one(&mut *tx)
    .await?;
    let (pack_id, pack_public_id): (i64, Uuid) = sqlx::query_as(
        "insert into app.context_packs (
           workspace_id, project_id, source_node_id, target_node_id, target_agent_profile_id,
           task_kind, objective, content, version
         ) values ($1,$2,$3,$4,$5,'technical-delivery-plan',$6,$7,$8)
         returning id, public_id",
    )
    .bind(project.workspace_id)
    .bind(project.id)
    .bind(product_node_id)
    .bind(tech_node_id)
    .bind(tech_profile_id)
    .bind(&project.objective)
    .bind(&content)
    .bind(version)
    .fetch_one(&mut *tx)
    .await?;
    for item in &knowledge {
        let (entry_id, version_id): (i64, i64) = sqlx::query_as(
            "select k.id, v.id from app.knowledge_entries k
             join app.knowledge_entry_versions v on v.knowledge_entry_id = k.id
             where k.public_id = $1 and v.public_id = $2",
        )
        .bind(item.public_id)
        .bind(item.version_public_id)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            "insert into app.context_pack_sources (
               context_pack_id, knowledge_entry_id, knowledge_entry_version_id, source_role, included_reason
             ) values ($1,$2,$3,$4,'Version confirmée pertinente pour le plan Tech')",
        )
        .bind(pack_id)
        .bind(entry_id)
        .bind(version_id)
        .bind(&item.entry_type)
        .execute(&mut *tx)
        .await?;
    }
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
    tx.commit().await?;
    Ok(HandoffView {
        public_id,
        context_pack_public_id: pack_public_id,
        target_session_public_id,
        status: "completed".into(),
        context_pack: content,
    })
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

#[derive(sqlx::FromRow)]
struct RequirementRecord {
    knowledge_id: i64,
    knowledge_public_id: Uuid,
    version_id: i64,
    version_public_id: Uuid,
    title: String,
    statement: String,
}

pub async fn generate_technical_plan(
    state: &AppState,
    project_public_id: Uuid,
    input: GenerateTechnicalPlan,
) -> AppResult<DeliverableSummary> {
    let project = project_by_public(&state.pool, state.workspace_id, project_public_id).await?;
    let session = session_by_public(&state.pool, state.workspace_id, input.session_id).await?;
    if session.project_id != project.id || session.node_key != "tech" {
        return Err(AppError::Invalid(
            "a Tech handoff session with a ContextPack is required".into(),
        ));
    }
    let pack_id = session.context_pack_id.ok_or_else(|| {
        AppError::Invalid("a Tech handoff session with a ContextPack is required".into())
    })?;
    let requirements = sqlx::query_as::<_, RequirementRecord>(
        "select k.id as knowledge_id, k.public_id as knowledge_public_id,
                v.id as version_id, v.public_id as version_public_id, v.title, v.statement
         from app.knowledge_entries k
         join app.knowledge_entry_versions v
           on v.knowledge_entry_id = k.id and v.version_number = k.latest_version
         where k.project_id = $1 and k.entry_type = 'requirement' and k.status = 'confirmed'
         order by k.id",
    )
    .bind(project.id)
    .fetch_all(&state.pool)
    .await?;
    if requirements.is_empty() {
        return Err(AppError::Invalid(
            "at least one confirmed requirement is required".into(),
        ));
    }
    let content = json!({
        "architecture": "API Rust/Axum transactionnelle, graphe PostgreSQL privé et client React/Tauri sans secret.",
        "delivery_slices": [
            "Ledger de crédits idempotent",
            "Projection de solde et règles de validité",
            "Observabilité, migration et reprise"
        ],
        "risks": ["Conflit entre durée métier et politique de purge", "Preuve automatisée encore partielle"],
        "validation": ["Tests de contrat API", "Tests PostgreSQL de reprise", "Parcours E2E desktop et mobile"],
        "coverage": requirements.iter().map(|requirement| json!({
            "requirement_public_id": requirement.knowledge_public_id,
            "version_public_id": requirement.version_public_id,
            "requirement": requirement.statement,
            "status": "partial"
        })).collect::<Vec<_>>()
    });
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "update app.deliverables set status = 'superseded'
         where project_id = $1 and deliverable_type = 'technical-delivery-plan' and status <> 'superseded'",
    )
    .bind(project.id)
    .execute(&mut *tx)
    .await?;
    let contract_id: i64 = sqlx::query_scalar(
        "select id from app.deliverable_contracts
         where template_id = $1 and contract_key = 'technical-delivery-plan'",
    )
    .bind(project.template_id)
    .fetch_one(&mut *tx)
    .await?;
    let deliverable = sqlx::query_as::<_, DeliverableSummary>(
        "insert into app.deliverables (
           workspace_id, project_id, context_node_id, contract_id, source_session_id,
           source_context_pack_id, deliverable_type, title, summary, content, status,
           coverage_status, committed_at
         ) values ($1,$2,$3,$4,$5,$6,'technical-delivery-plan',$7,$8,$9,'committed','partial',now())
         returning public_id, deliverable_type, title, summary, content, status,
                   coverage_status, version, updated_at",
    )
    .bind(project.workspace_id)
    .bind(project.id)
    .bind(session.context_node_id)
    .bind(contract_id)
    .bind(session.id)
    .bind(session.context_pack_id)
    .bind(format!("Technical Delivery Plan — {}", project.name))
    .bind("Plan exécutable relié aux exigences confirmées ; preuves à compléter.")
    .bind(&content)
    .fetch_one(&mut *tx)
    .await?;
    let deliverable_id: i64 =
        sqlx::query_scalar("select id from app.deliverables where public_id = $1")
            .bind(deliverable.public_id)
            .fetch_one(&mut *tx)
            .await?;
    let sections = [
        (
            "architecture",
            "Architecture",
            content["architecture"].as_str().unwrap_or_default(),
        ),
        (
            "delivery_slices",
            "Découpage de livraison",
            "Ledger → projection → observabilité et reprise.",
        ),
        (
            "risks",
            "Risques",
            "La politique de purge à 90 jours peut contredire la règle Produit.",
        ),
        (
            "validation",
            "Validation",
            "Contrats API, intégration PostgreSQL, Playwright et smoke Tauri.",
        ),
        (
            "coverage",
            "Couverture",
            "Chaque exigence possède un état ; les preuves restent partielles.",
        ),
    ];
    let mut section_ids = Vec::new();
    for (ordinal, (key, title, body)) in (0_i32..).zip(sections) {
        let (id, public_id): (i64, Uuid) = sqlx::query_as(
            "insert into app.deliverable_sections (
               deliverable_id, section_key, title, body, ordinal, source_data
             ) values ($1,$2,$3,$4,$5,$6) returning id, public_id",
        )
        .bind(deliverable_id)
        .bind(key)
        .bind(title)
        .bind(body)
        .bind(ordinal)
        .bind(json!({"requirement_version_ids": requirements.iter().map(|item| item.version_public_id).collect::<Vec<_>>() }))
        .fetch_one(&mut *tx)
        .await?;
        section_ids.push((key, id, public_id));
    }
    let (_, coverage_section_id, _) = section_ids
        .iter()
        .find(|(key, _, _)| *key == "coverage")
        .ok_or_else(|| AppError::Internal("coverage section missing".into()))?;
    for requirement in &requirements {
        let evidence_public_id: Uuid = sqlx::query_scalar(
            "insert into app.evidences (
               workspace_id, project_id, requirement_entry_id, requirement_version_id,
               deliverable_id, deliverable_section_id, evidence_type, title, description,
               source_reference
             ) values ($1,$2,$3,$4,$5,$6,'analysis',$7,$8,$9) returning public_id",
        )
        .bind(project.workspace_id)
        .bind(project.id)
        .bind(requirement.knowledge_id)
        .bind(requirement.version_id)
        .bind(deliverable_id)
        .bind(*coverage_section_id)
        .bind(format!("Analyse — {}", requirement.title))
        .bind("Le plan identifie la cible, mais une preuve automatisée reste à attacher.")
        .bind(format!("deliverable:{}#coverage", deliverable.public_id))
        .fetch_one(&mut *tx)
        .await?;
        let evidence_id: i64 =
            sqlx::query_scalar("select id from app.evidences where public_id = $1")
                .bind(evidence_public_id)
                .fetch_one(&mut *tx)
                .await?;
        sqlx::query(
            "insert into app.requirement_coverage (
               project_id, requirement_entry_id, requirement_version_id, deliverable_id,
               deliverable_section_id, evidence_id, status, explanation
             ) values ($1,$2,$3,$4,$5,$6,'partial',
               'Traçabilité documentaire présente ; test exécutable manquant.')",
        )
        .bind(project.id)
        .bind(requirement.knowledge_id)
        .bind(requirement.version_id)
        .bind(deliverable_id)
        .bind(*coverage_section_id)
        .bind(evidence_id)
        .execute(&mut *tx)
        .await?;
    }
    let task_id: i64 = sqlx::query_scalar(
        "insert into app.tasks (workspace_id, project_id, context_pack_id, contract_id, title, status)
         values ($1,$2,$3,$4,$5,'completed') returning id",
    )
    .bind(project.workspace_id)
    .bind(project.id)
    .bind(pack_id)
    .bind(contract_id)
    .bind(format!("Produire le plan Tech — {}", project.name))
    .fetch_one(&mut *tx)
    .await?;
    let execution_id: i64 = sqlx::query_scalar(
        "insert into app.executions (
           workspace_id, project_id, task_id, executor_key, status, result, started_at, completed_at
         ) values ($1,$2,$3,'document-simulator','completed',$4,now(),now()) returning id",
    )
    .bind(project.workspace_id)
    .bind(project.id)
    .bind(task_id)
    .bind(json!({"deliverable_public_id": deliverable.public_id}))
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query(
        "insert into app.execution_events (execution_id, event_type, sequence_number, payload)
         values ($1,'deliverable.committed',1,$2)",
    )
    .bind(execution_id)
    .bind(json!({"deliverable_public_id": deliverable.public_id}))
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "insert into app.artifacts (
           workspace_id, project_id, execution_id, deliverable_id, artifact_type, title, reference, metadata
         ) values ($1,$2,$3,$4,'document','Technical Delivery Plan',$5,$6)",
    )
    .bind(project.workspace_id)
    .bind(project.id)
    .bind(execution_id)
    .bind(deliverable_id)
    .bind(format!("aicenter://deliverables/{}", deliverable.public_id))
    .bind(json!({"executor": "document-simulator"}))
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
                   insight_id, source_role, object_kind, object_public_id, knowledge_entry_version_id
                 ) values ($1,'unproven_requirement','knowledge_entry_version',$2,$3)",
            )
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
    tx.commit().await?;
    Ok(deliverable)
}

pub async fn coverage(state: &AppState, project_public_id: Uuid) -> AppResult<CoverageView> {
    let project = project_by_public(&state.pool, state.workspace_id, project_public_id).await?;
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
    .fetch_all(&state.pool)
    .await?;
    Ok(CoverageView {
        total: items.len(),
        covered: items.iter().filter(|item| item.status == "covered").count(),
        partial: items.iter().filter(|item| item.status == "partial").count(),
        missing: items.iter().filter(|item| item.status == "missing").count(),
        items,
    })
}

pub async fn list_insights(state: &AppState) -> AppResult<Vec<InsightSummary>> {
    Ok(sqlx::query_as::<_, InsightSummary>(
        "select i.public_id, i.insight_type, i.status, i.severity,
                i.confidence::double precision as confidence, i.title, i.explanation,
                i.resolution_justification, i.detected_at, i.updated_at
         from app.insights i join app.workspaces w on w.id = i.workspace_id
         where w.public_id = $1 order by
           case i.severity when 'blocking' then 0 when 'warning' then 1 else 2 end,
           i.detected_at desc, i.id desc",
    )
    .bind(state.workspace_id)
    .fetch_all(&state.pool)
    .await?)
}

pub async fn insight_detail(state: &AppState, public_id: Uuid) -> AppResult<InsightDetail> {
    let insight = sqlx::query_as::<_, InsightSummary>(
        "select i.public_id, i.insight_type, i.status, i.severity,
                i.confidence::double precision as confidence, i.title, i.explanation,
                i.resolution_justification, i.detected_at, i.updated_at
         from app.insights i join app.workspaces w on w.id = i.workspace_id
         where w.public_id = $1 and i.public_id = $2",
    )
    .bind(state.workspace_id)
    .bind(public_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    let sources = sqlx::query_as::<_, InsightSourceView>(
        "select s.source_role, s.object_kind, s.object_public_id,
                v.title as version_title, v.statement as version_statement
         from app.insight_sources s
         join app.insights i on i.id = s.insight_id
         left join app.knowledge_entry_versions v on v.id = s.knowledge_entry_version_id
         where i.public_id = $1 order by s.id",
    )
    .bind(public_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(InsightDetail { insight, sources })
}

pub async fn act_on_insight(
    state: &AppState,
    public_id: Uuid,
    input: InsightAction,
) -> AppResult<InsightDetail> {
    let justification = input.justification.trim();
    if justification.is_empty() {
        return Err(AppError::Invalid("a justification is required".into()));
    }
    let next_status = match input.action {
        InsightDecision::Accept => "accepted",
        InsightDecision::Dismiss => "dismissed",
        InsightDecision::Resolve => "resolved",
    };
    let mut tx = state.pool.begin().await?;
    let row: Option<(i64, i64, String)> = sqlx::query_as(
        "select i.workspace_id, i.project_id, i.status
         from app.insights i join app.workspaces w on w.id = i.workspace_id
         where w.public_id = $1 and i.public_id = $2 for update",
    )
    .bind(state.workspace_id)
    .bind(public_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (workspace_id, project_id, before_status) = row.ok_or(AppError::NotFound)?;
    sqlx::query(
        "update app.insights set status = $2, resolution_justification = $3,
                resolved_by_actor_id = $4, updated_at = now() where public_id = $1",
    )
    .bind(public_id)
    .bind(next_status)
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
    tx.commit().await?;
    insight_detail(state, public_id).await
}

pub async fn project_history(
    state: &AppState,
    project_public_id: Uuid,
) -> AppResult<Vec<HistoryEvent>> {
    let project = project_by_public(&state.pool, state.workspace_id, project_public_id).await?;
    Ok(sqlx::query_as::<_, HistoryEvent>(
        "select public_id, action, object_kind, object_public_id,
                before_state, after_state, occurred_at
         from app.audit_events where project_id = $1
         order by occurred_at desc, id desc limit 200",
    )
    .bind(project.id)
    .fetch_all(&state.pool)
    .await?)
}

type KnowledgeRevisionRow = (i64, i64, i64, i64, i32, String, String, String);

pub async fn revise_knowledge(
    state: &AppState,
    knowledge_public_id: Uuid,
    input: ReviseKnowledge,
) -> AppResult<RevisionResult> {
    let statement = input.statement.trim();
    if statement.is_empty() {
        return Err(AppError::Invalid("knowledge statement is required".into()));
    }
    let mut tx = state.pool.begin().await?;
    let row: Option<KnowledgeRevisionRow> = sqlx::query_as(
        "select k.id, k.workspace_id, k.project_id, k.context_node_id, k.latest_version,
                v.entry_type, v.title, v.rationale
         from app.knowledge_entries k
         join app.workspaces w on w.id = k.workspace_id
         join app.knowledge_entry_versions v
           on v.knowledge_entry_id = k.id and v.version_number = k.latest_version
         where w.public_id = $1 and k.public_id = $2 for update of k",
    )
    .bind(state.workspace_id)
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
    invalidate_projections(&mut tx, project_id).await?;
    let insight_ids = detect_reference_conflict(&mut tx, workspace_id, project_id).await?;
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
    tx.commit().await?;
    Ok(RevisionResult {
        knowledge_public_id,
        version_public_id,
        version_number: next_version,
        graph_version,
        insight_ids,
    })
}

async fn invalidate_projections(
    tx: &mut Transaction<'_, Postgres>,
    project_id: i64,
) -> AppResult<()> {
    sqlx::query("update app.context_packs set status = 'stale', invalidated_at = now() where project_id = $1 and status = 'current'")
        .bind(project_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query("update app.deliverables set status = 'stale', stale_at = now() where project_id = $1 and status = 'committed'")
        .bind(project_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn detect_reference_conflict(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: i64,
    project_id: i64,
) -> AppResult<Vec<Uuid>> {
    let product: Option<(Uuid, i64)> = sqlx::query_as(
        "select v.public_id, v.id
         from app.knowledge_entry_versions v join app.context_nodes n on n.id = v.context_node_id
         join app.knowledge_entries k on k.id = v.knowledge_entry_id and k.latest_version = v.version_number
         where v.project_id = $1 and n.node_key = 'product' and v.statement ilike '%expir%'
           and v.statement ilike '%jamais%' and k.status = 'confirmed'
         order by v.id desc limit 1",
    )
    .bind(project_id)
    .fetch_optional(&mut **tx)
    .await?;
    let tech: Option<(Uuid, i64)> = sqlx::query_as(
        "select v.public_id, v.id
         from app.knowledge_entry_versions v join app.context_nodes n on n.id = v.context_node_id
         join app.knowledge_entries k on k.id = v.knowledge_entry_id and k.latest_version = v.version_number
         where v.project_id = $1 and n.node_key = 'tech'
           and (v.statement ilike '%90 jour%' or v.statement ilike '%purge%')
           and v.statement not ilike '%sans purge%'
           and v.statement not ilike '%aucune purge%'
           and v.statement not ilike '%ne purge pas%'
           and k.status = 'confirmed'
         order by v.id desc limit 1",
    )
    .bind(project_id)
    .fetch_optional(&mut **tx)
    .await?;
    let (Some((product_public_id, product_version_id)), Some((tech_public_id, tech_version_id))) =
        (product, tech)
    else {
        sqlx::query(
            "update app.insights set status = 'resolved',
                    resolution_justification = 'Résolu automatiquement après réévaluation des versions courantes',
                    updated_at = now()
             where project_id = $1 and insight_type = 'contradiction'
               and status in ('candidate','open','accepted')",
        )
        .bind(project_id)
        .execute(&mut **tx)
        .await?;
        return Ok(Vec::new());
    };
    let existing: Option<Uuid> = sqlx::query_scalar(
        "select public_id from app.insights
         where project_id = $1 and insight_type = 'contradiction'
           and status in ('candidate','open','accepted') limit 1",
    )
    .bind(project_id)
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(public_id) = existing {
        return Ok(vec![public_id]);
    }
    let (insight_id, public_id): (i64, Uuid) = sqlx::query_as(
        "insert into app.insights (
           workspace_id, project_id, insight_type, status, severity, confidence, title, explanation
         ) values ($1,$2,'contradiction','open','blocking',0.990,
           'Expiration des crédits incohérente',
           'La règle Produit conserve les crédits sans expiration tandis que la règle Tech prévoit une purge après 90 jours.')
         returning id, public_id",
    )
    .bind(workspace_id)
    .bind(project_id)
    .fetch_one(&mut **tx)
    .await?;
    for (role, version_public_id, version_id) in [
        ("product_rule", product_public_id, product_version_id),
        ("tech_rule", tech_public_id, tech_version_id),
    ] {
        sqlx::query(
            "insert into app.insight_sources (
               insight_id, source_role, object_kind, object_public_id, knowledge_entry_version_id
             ) values ($1,$2,'knowledge_entry_version',$3,$4)",
        )
        .bind(insight_id)
        .bind(role)
        .bind(version_public_id)
        .bind(version_id)
        .execute(&mut **tx)
        .await?;
    }
    Ok(vec![public_id])
}

async fn project_by_public(
    pool: &PgPool,
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
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}

async fn session_by_public(
    pool: &PgPool,
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
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}

async fn knowledge_for_project(pool: &PgPool, project_id: i64) -> AppResult<Vec<KnowledgeSummary>> {
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
    .fetch_all(pool)
    .await?)
}

async fn insights_for_project(pool: &PgPool, project_id: i64) -> AppResult<Vec<InsightSummary>> {
    Ok(sqlx::query_as::<_, InsightSummary>(
        "select public_id, insight_type, status, severity, confidence::double precision as confidence,
                title, explanation, resolution_justification, detected_at, updated_at
         from app.insights where project_id = $1 order by detected_at desc, id desc",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?)
}

async fn latest_gate(pool: &PgPool, project_id: i64) -> AppResult<Option<GateResult>> {
    let row: Option<(Uuid, String, i64, Value)> = sqlx::query_as(
        "select public_id, status, graph_version, evaluation
         from app.gates where project_id = $1 and gate_key = 'product-ready'
         order by graph_version desc, id desc limit 1",
    )
    .bind(project_id)
    .fetch_optional(pool)
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
