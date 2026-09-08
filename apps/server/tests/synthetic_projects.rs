//! Two fictional projects exercise real services and disposable PostgreSQL/RLS.
//! `DeterministicEngine` is a provider double, not a claim of model quality.

use std::{collections::HashSet, sync::Arc};

use ai_center_server::{
    agent::DeterministicEngine,
    error::AppError,
    models::{
        CompileContextPack, ContextPackSummary, CreateHandoff, CreateProject, CreateSession,
        DecideProposals, DeliverableSummary, GenerateTechnicalPlan, HandoffView,
        InsightResolutionMutation, ProposalDecision, ResolveInsight, SendMessage,
    },
    service::{self, AppState},
};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[derive(Deserialize)]
struct FictionalProject {
    synthetic: bool,
    name: String,
    objective: String,
    rule: String,
    unconfirmed_note: String,
    historical_technical_note: String,
    contradictory_rule: Option<String>,
    corrected_rule: Option<String>,
}

struct ProjectRun {
    fixture: FictionalProject,
    project_id: Uuid,
    product_session_id: Uuid,
    pack: ContextPackSummary,
    handoff: HandoffView,
    plan: DeliverableSummary,
}

#[tokio::test]
async fn two_fictional_projects_keep_confirmation_provenance_and_revision_isolated() -> Result<()> {
    let state = isolated_state().await?;
    let [library_fixture, workshop_fixture]: [FictionalProject; 2] =
        serde_json::from_str(include_str!("fixtures/synthetic-projects.json"))?;
    let library = run_to_plan(&state, library_fixture).await?;
    let workshop = run_to_plan(&state, workshop_fixture).await?;

    assert_project_isolation(&state, &library, &workshop).await?;
    assert_project_isolation(&state, &workshop, &library).await?;
    assert_pack_unchanged(&state, &library.pack, "current").await?;
    let workshop_before =
        serde_json::to_value(service::snapshot(&state, workshop.project_id).await?)?;

    revise_library_contradiction(&state, &library).await?;

    assert_pack_unchanged(&state, &workshop.pack, "current").await?;
    let workshop_after =
        serde_json::to_value(service::snapshot(&state, workshop.project_id).await?)?;
    assert_eq!(
        workshop_before, workshop_after,
        "the other project's graph, plan and insights must remain unchanged"
    );
    assert_missing_coverage(&state, &workshop).await?;
    state.pool.close().await;
    Ok(())
}

async fn isolated_state() -> Result<AppState> {
    let raw_url = std::env::var("DATABASE_URL").context(
        "run synthetic_projects through the guarded integration stack; DATABASE_URL is required",
    )?;
    let database = url::Url::parse(&raw_url)
        .map_err(|_| anyhow::anyhow!("invalid isolated integration database URL"))?;
    ensure!(
        database.scheme() == "postgresql"
            && database.username() == "ai_center_runtime"
            && database.host_str() == Some("127.0.0.1")
            && database.port() == Some(55322)
            && database.path() == "/postgres"
            && database.query().is_none()
            && database.fragment().is_none(),
        "synthetic projects require the isolated runtime database on loopback port 55322"
    );
    ensure!(
        std::env::var("AI_CENTER_EXPECT_DATABASE_ROLE").as_deref() == Ok("ai_center_runtime")
            && std::env::var("AI_CENTER_AGENT_MODE").as_deref() == Ok("deterministic"),
        "explicit runtime role and deterministic integration mode are required"
    );
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&raw_url)
        .await?;
    let (role, superuser, bypass_rls): (String, bool, bool) = sqlx::query_as(
        "select current_user::text, rolsuper, rolbypassrls from pg_roles where rolname = current_user",
    )
    .fetch_one(&pool)
    .await?;
    ensure!(
        role == "ai_center_runtime" && !superuser && !bypass_rls,
        "tests require runtime RLS enforcement"
    );
    let workspace_id = "10000000-0000-0000-0000-000000000001".parse()?;
    let actor_id = "00000000-0000-0000-0000-000000000001".parse()?;
    let (workspace_internal_id, workspace_role): (i64, String) =
        sqlx::query_as("select workspace_id, role from app.authorize_workspace_member($1, $2)")
            .bind(workspace_id)
            .bind(actor_id)
            .fetch_one(&pool)
            .await?;
    Ok(AppState {
        pool,
        engine: Arc::new(DeterministicEngine),
        workspace_id,
        workspace_internal_id: Some(workspace_internal_id),
        workspace_role,
        actor_id,
        agent_mode: "deterministic",
        steward_trigger: None,
        providers: None,
    })
}

#[allow(clippy::too_many_lines)]
async fn run_to_plan(state: &AppState, fixture: FictionalProject) -> Result<ProjectRun> {
    ensure!(
        fixture.synthetic,
        "only explicitly fictional fixtures are allowed"
    );
    let project = service::create_project(
        state,
        CreateProject {
            name: format!("[FICTIF] {} {}", fixture.name, Uuid::new_v4()),
            objective: fixture.objective.clone(),
        },
    )
    .await?;
    let product = service::create_session(
        state,
        project.public_id,
        CreateSession {
            node_key: "product".into(),
            title: Some(format!("Cadrage fictif — {}", fixture.name)),
        },
    )
    .await?;
    let product_session_id = product.session.public_id;
    let proposed = service::send_message(
        state,
        product_session_id,
        SendMessage {
            content: fixture.rule.clone(),
            client_message_id: Uuid::new_v4(),
        },
    )
    .await?;
    assert_eq!(proposed.proposals.len(), 3);
    assert!(
        proposed
            .proposals
            .iter()
            .all(|item| item.status == "proposed")
    );
    let gate = service::evaluate_product_gate(state, project.public_id).await?;
    assert_eq!(gate.status, "blocked");
    assert_eq!(gate.missing.len(), 3);
    assert!(
        service::snapshot(state, project.public_id)
            .await?
            .knowledge
            .is_empty()
    );
    assert!(
        matches!(compile_pack(state, project.public_id, product_session_id).await,
        Err(AppError::Invalid(message)) if message.contains("ProductReadyGate"))
    );

    let confirmed = service::decide_proposals(
        state,
        product_session_id,
        DecideProposals {
            proposal_ids: proposed
                .proposals
                .iter()
                .map(|item| item.public_id)
                .collect(),
            decision: ProposalDecision::Confirm,
        },
    )
    .await?;
    assert_eq!(confirmed.confirmed.len(), 3);
    let draft = service::send_message(
        state,
        product_session_id,
        SendMessage {
            content: fixture.unconfirmed_note.clone(),
            client_message_id: Uuid::new_v4(),
        },
    )
    .await?;
    assert_eq!(
        draft
            .proposals
            .iter()
            .filter(|item| item.status == "proposed")
            .count(),
        3
    );
    assert_eq!(
        service::snapshot(state, project.public_id)
            .await?
            .project
            .graph_version,
        confirmed.graph_version
    );

    // Confirm the fixture's historical knowledge through services. Its
    // presence in the graph must not make it mandatory in a Product-to-Tech pack.
    let initial_pack = compile_pack(state, project.public_id, product_session_id).await?;
    let history = service::create_handoff(
        state,
        project.public_id,
        CreateHandoff {
            source_session_id: product_session_id,
            context_pack_id: initial_pack.public_id,
        },
    )
    .await?;
    let historical = service::send_message(
        state,
        history.target_session_public_id,
        SendMessage {
            content: fixture.historical_technical_note.clone(),
            client_message_id: Uuid::new_v4(),
        },
    )
    .await?;
    service::decide_proposals(
        state,
        history.target_session_public_id,
        DecideProposals {
            proposal_ids: historical
                .proposals
                .iter()
                .map(|item| item.public_id)
                .collect(),
            decision: ProposalDecision::Confirm,
        },
    )
    .await?;
    let gate = service::evaluate_product_gate(state, project.public_id).await?;
    assert_eq!(gate.status, "passed");
    assert!(gate.missing.is_empty());
    let brief = service::generate_feature_brief(state, project.public_id).await?;
    assert_eq!(brief.deliverable_type, "feature-brief");
    assert_eq!(brief.status, "committed");
    assert!(brief.content.to_string().contains(&fixture.rule));
    assert!(
        !brief
            .content
            .to_string()
            .contains(&fixture.unconfirmed_note)
    );

    let pack = compile_pack(state, project.public_id, product_session_id).await?;
    let handoff = service::create_handoff(
        state,
        project.public_id,
        CreateHandoff {
            source_session_id: product_session_id,
            context_pack_id: pack.public_id,
        },
    )
    .await?;
    assert_eq!(handoff.status, "completed");
    assert_eq!(handoff.context_pack_public_id, pack.public_id);
    let plan = service::generate_technical_plan(
        state,
        project.public_id,
        GenerateTechnicalPlan {
            session_id: handoff.target_session_public_id,
        },
    )
    .await?;
    assert_eq!(plan.deliverable_type, "technical-delivery-plan");
    assert_eq!(plan.status, "committed");
    assert!(plan.content.to_string().contains(&fixture.rule));
    let run = ProjectRun {
        fixture,
        project_id: project.public_id,
        product_session_id,
        pack,
        handoff,
        plan,
    };
    assert_pack_provenance(state, &run).await?;
    assert_missing_coverage(state, &run).await?;
    Ok(run)
}

async fn compile_pack(
    state: &AppState,
    project_id: Uuid,
    source_session_id: Uuid,
) -> Result<ContextPackSummary, AppError> {
    service::compile_context_pack(
        state,
        project_id,
        CompileContextPack {
            source_session_id,
            task_kind: "technical-delivery-plan".into(),
            token_budget: Some(12_000),
        },
    )
    .await
}

async fn assert_pack_provenance(state: &AppState, run: &ProjectRun) -> Result<()> {
    let snapshot = service::snapshot(state, run.project_id).await?;
    let current_versions = snapshot
        .knowledge
        .iter()
        .map(|item| item.version_public_id)
        .collect::<HashSet<_>>();
    let pack = &run.pack;
    assert_eq!(pack.status, "current");
    assert_eq!(pack.source_graph_version, snapshot.project.graph_version);
    assert!(pack.token_count > 0 && pack.token_count <= pack.token_budget);
    assert_eq!(pack.selection_mode, "deterministic");
    assert!(pack.content.to_string().contains(&run.fixture.rule));
    assert!(
        !pack
            .content
            .to_string()
            .contains(&run.fixture.unconfirmed_note)
    );
    assert!(
        !pack
            .content
            .to_string()
            .contains(&run.fixture.historical_technical_note)
    );
    let knowledge = pack.content["knowledge"]
        .as_array()
        .context("pack knowledge")?;
    assert_eq!(knowledge.len(), 3);
    assert_eq!(pack.selection_items.len(), 4);
    let included = uuids(knowledge, "version_public_id")?;
    let provenance = pack.content["provenance"]
        .as_array()
        .context("pack provenance")?;
    assert_eq!(included, uuids(provenance, "version_public_id")?);
    assert!(included.is_subset(&current_versions));
    for source in knowledge {
        let version: Uuid = serde_json::from_value(source["version_public_id"].clone())?;
        let canonical = snapshot
            .knowledge
            .iter()
            .find(|item| item.version_public_id == version)
            .context("confirmed source")?;
        assert_eq!(source["knowledge_public_id"], json!(canonical.public_id));
        assert_eq!(source["version_number"], canonical.version_number);
        assert_eq!(source["statement"], canonical.statement);
        let trace = provenance
            .iter()
            .find(|item| item["version_public_id"] == source["version_public_id"])
            .context("source provenance")?;
        assert_eq!(trace["knowledge_public_id"], source["knowledge_public_id"]);
        assert_eq!(trace["reason_code"], "contract_required");
    }
    let historical = snapshot
        .knowledge
        .iter()
        .find(|item| item.statement == run.fixture.historical_technical_note)
        .context("confirmed history")?;
    for item in &pack.selection_items {
        assert!(current_versions.contains(&item.candidate_public_id));
        if item.candidate_public_id == historical.version_public_id {
            assert_eq!(item.decision, "excluded");
            assert_eq!(item.reason_code, "historical_tech_unrelated");
            assert!(!item.is_mandatory);
        } else {
            assert_eq!(item.decision, "included");
            assert_eq!(item.reason_code, "contract_required");
            assert!(item.is_mandatory);
        }
    }
    let (mime, exported) = service::export_context_pack(state, pack.public_id, "json").await?;
    assert_eq!(mime, "application/json; charset=utf-8");
    let exported: Value = serde_json::from_str(&exported)?;
    assert_eq!(exported, serde_json::to_value(pack)?);
    assert_eq!(
        pack.content_hash,
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&exported["content"])?)
        )
    );
    Ok(())
}

fn uuids(items: &[Value], field: &str) -> Result<HashSet<Uuid>> {
    items
        .iter()
        .map(|item| serde_json::from_value(item[field].clone()).map_err(Into::into))
        .collect()
}

async fn assert_project_isolation(
    state: &AppState,
    own: &ProjectRun,
    foreign: &ProjectRun,
) -> Result<()> {
    let own_sources = uuids(
        own.pack.content["knowledge"]
            .as_array()
            .context("own knowledge")?,
        "version_public_id",
    )?;
    let foreign_sources = uuids(
        foreign.pack.content["knowledge"]
            .as_array()
            .context("foreign knowledge")?,
        "version_public_id",
    )?;
    assert!(own_sources.is_disjoint(&foreign_sources));
    assert!(!own.pack.content.to_string().contains(&foreign.fixture.rule));
    assert!(!own.plan.content.to_string().contains(&foreign.fixture.rule));
    for section in own.plan.content["delivery_slices"]
        .as_array()
        .context("delivery slices")?
    {
        let sources: HashSet<Uuid> = serde_json::from_value(section["source_version_ids"].clone())?;
        assert!(!sources.is_empty());
        assert!(sources.is_subset(&own_sources));
    }
    let before = service::snapshot(state, own.project_id).await?;
    assert!(matches!(
        service::create_handoff(
            state,
            own.project_id,
            CreateHandoff {
                source_session_id: own.product_session_id,
                context_pack_id: foreign.pack.public_id,
            }
        )
        .await,
        Err(AppError::NotFound)
    ));
    assert!(
        matches!(compile_pack(state, own.project_id, foreign.product_session_id).await,
        Err(AppError::Invalid(message)) if message.contains("this project"))
    );
    assert!(matches!(
        service::generate_technical_plan(
            state,
            own.project_id,
            GenerateTechnicalPlan {
                session_id: foreign.handoff.target_session_public_id,
            }
        )
        .await,
        Err(AppError::Invalid(_))
    ));
    let after = service::snapshot(state, own.project_id).await?;
    assert_eq!(after.sessions.len(), before.sessions.len());
    assert_eq!(after.deliverables.len(), before.deliverables.len());
    assert_eq!(after.project.graph_version, before.project.graph_version);
    Ok(())
}

async fn assert_missing_coverage(state: &AppState, run: &ProjectRun) -> Result<()> {
    let coverage = service::coverage(state, run.project_id).await?;
    assert!(coverage.total > 0);
    assert_eq!(coverage.covered, 0);
    assert_eq!(coverage.partial, 0);
    assert_eq!(coverage.missing, coverage.total);
    let snapshot = service::snapshot(state, run.project_id).await?;
    let requirements = snapshot
        .knowledge
        .iter()
        .filter(|item| item.entry_type == "requirement")
        .map(|item| item.version_public_id)
        .collect::<HashSet<_>>();
    for item in coverage.items {
        assert_eq!(item.status, "missing");
        assert!(item.evidence_public_id.is_none());
        assert!(requirements.contains(&item.requirement_version_public_id));
    }
    assert_eq!(run.plan.coverage_status, "missing");
    Ok(())
}

async fn assert_pack_unchanged(
    state: &AppState,
    original: &ContextPackSummary,
    status: &str,
) -> Result<()> {
    let current = service::context_pack(state, original.public_id).await?;
    assert_eq!(current.status, status);
    assert_eq!(current.content, original.content);
    assert_eq!(current.content_hash, original.content_hash);
    assert_eq!(current.version, original.version);
    assert_eq!(current.source_graph_version, original.source_graph_version);
    assert_eq!(current.compiled_at, original.compiled_at);
    assert_eq!(
        serde_json::to_value(&current.selection_items)?,
        serde_json::to_value(&original.selection_items)?
    );
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn revise_library_contradiction(state: &AppState, library: &ProjectRun) -> Result<()> {
    let contradictory_rule = library
        .fixture
        .contradictory_rule
        .as_deref()
        .context("library contradiction")?;
    let corrected_rule = library
        .fixture
        .corrected_rule
        .as_deref()
        .context("library correction")?;
    let turn = service::send_message(
        state,
        library.handoff.target_session_public_id,
        SendMessage {
            content: contradictory_rule.into(),
            client_message_id: Uuid::new_v4(),
        },
    )
    .await?;
    let commit = service::decide_proposals(
        state,
        library.handoff.target_session_public_id,
        DecideProposals {
            proposal_ids: turn
                .proposals
                .iter()
                .filter(|item| item.status == "proposed")
                .map(|item| item.public_id)
                .collect(),
            decision: ProposalDecision::Confirm,
        },
    )
    .await?;
    assert_eq!(commit.confirmed.len(), 1);
    let snapshot = service::snapshot(state, library.project_id).await?;
    let conflicting = snapshot
        .knowledge
        .iter()
        .find(|item| item.statement == contradictory_rule)
        .context("confirmed contradiction")?;
    let insight_id = *commit
        .insight_ids
        .first()
        .context("blocking Steward insight")?;
    let insight = service::insight_detail(state, insight_id).await?;
    assert_eq!(insight.insight.severity, "blocking");
    assert_eq!(insight.insight.project_public_id, library.project_id);
    assert_eq!(insight.sources.len(), 2);
    assert!(
        insight
            .sources
            .iter()
            .any(|item| item.version_public_id == Some(conflicting.version_public_id))
    );
    let current_versions = snapshot
        .knowledge
        .iter()
        .map(|item| item.version_public_id)
        .collect::<HashSet<_>>();
    assert!(insight.sources.iter().all(|item| {
        item.version_public_id
            .is_some_and(|id| current_versions.contains(&id))
    }));
    assert_pack_unchanged(state, &library.pack, "stale").await?;
    assert!(
        matches!(service::generate_technical_plan(state, library.project_id, GenerateTechnicalPlan {
        session_id: library.handoff.target_session_public_id,
    }).await, Err(AppError::Conflict(message)) if message.contains("stale"))
    );
    assert!(
        matches!(service::create_handoff(state, library.project_id, CreateHandoff {
        source_session_id: library.product_session_id,
        context_pack_id: library.pack.public_id,
    }).await, Err(AppError::Conflict(message)) if message.contains("stale"))
    );

    let resolved = service::resolve_insight(
        state,
        library.project_id,
        insight_id,
        ResolveInsight {
            expected_graph_version: insight.insight.project_graph_version,
            justification:
                "Cas fictif : aligner la décision technique sur la conservation validée.".into(),
            mutations: vec![InsightResolutionMutation::ReviseKnowledge {
                knowledge_public_id: conflicting.public_id,
                expected_version_public_id: conflicting.version_public_id,
                statement: corrected_rule.into(),
                title: None,
                rationale: Some("La médiathèque conserve l'historique des prêts.".into()),
            }],
        },
    )
    .await?;
    assert_eq!(resolved.insight.status, "resolved");
    let revised = service::snapshot(state, library.project_id).await?;
    let corrected = revised
        .knowledge
        .iter()
        .find(|item| item.public_id == conflicting.public_id)
        .context("revised technical knowledge")?;
    assert_eq!(corrected.statement, corrected_rule);
    assert_eq!(corrected.version_number, conflicting.version_number + 1);
    assert_ne!(corrected.version_public_id, conflicting.version_public_id);
    assert!(revised.project.graph_version > snapshot.project.graph_version);
    let archived_insight = service::insight_detail(state, insight_id).await?;
    assert!(
        archived_insight
            .sources
            .iter()
            .any(
                |item| item.version_public_id == Some(conflicting.version_public_id)
                    && item.version_statement.as_deref() == Some(contradictory_rule)
            ),
        "the resolved insight must retain the original contradictory version"
    );
    let history = service::project_history(state, library.project_id).await?;
    let resolution = history
        .iter()
        .find(|event| {
            event.action == "insight.resolved_with_revision" && event.object_public_id == insight_id
        })
        .context("resolution audit")?;
    assert_eq!(
        resolution.before_state.as_ref().context("audit before")?["knowledge_version_public_id"],
        json!(conflicting.version_public_id)
    );
    assert_eq!(
        resolution.after_state.as_ref().context("audit after")?["knowledge_version_public_id"],
        json!(corrected.version_public_id)
    );
    assert_pack_unchanged(state, &library.pack, "stale").await?;

    let fresh_pack = compile_pack(state, library.project_id, library.product_session_id).await?;
    assert_eq!(fresh_pack.status, "current");
    assert_eq!(
        fresh_pack.source_graph_version,
        revised.project.graph_version
    );
    assert!(fresh_pack.version > library.pack.version);
    assert_ne!(fresh_pack.public_id, library.pack.public_id);
    assert_ne!(fresh_pack.content_hash, library.pack.content_hash);
    assert!(
        fresh_pack
            .content
            .to_string()
            .contains(&library.fixture.rule)
    );
    assert!(!fresh_pack.content.to_string().contains(contradictory_rule));
    assert!(
        fresh_pack
            .selection_items
            .iter()
            .any(
                |item| item.candidate_public_id == corrected.version_public_id
                    && item.decision == "excluded"
                    && !item.is_mandatory
            )
    );
    assert!(
        !fresh_pack
            .selection_items
            .iter()
            .any(|item| item.candidate_public_id == conflicting.version_public_id)
    );
    let handoff = service::create_handoff(
        state,
        library.project_id,
        CreateHandoff {
            source_session_id: library.product_session_id,
            context_pack_id: fresh_pack.public_id,
        },
    )
    .await?;
    let plan = service::generate_technical_plan(
        state,
        library.project_id,
        GenerateTechnicalPlan {
            session_id: handoff.target_session_public_id,
        },
    )
    .await?;
    assert_eq!(plan.status, "committed");
    assert!(plan.version > library.plan.version);
    assert!(plan.content.to_string().contains(&library.fixture.rule));
    assert!(!plan.content.to_string().contains(contradictory_rule));
    assert_eq!(plan.coverage_status, "missing");
    assert_missing_coverage(state, library).await?;
    let after = service::snapshot(state, library.project_id).await?;
    let original_plan = after
        .deliverables
        .iter()
        .find(|item| item.public_id == library.plan.public_id)
        .context("original technical plan")?;
    assert_eq!(original_plan.status, "superseded");
    assert_eq!(original_plan.content, library.plan.content);
    assert_pack_unchanged(state, &library.pack, "stale").await?;
    Ok(())
}
