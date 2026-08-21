use std::sync::Arc;

use ai_center_server::{
    agent::DeterministicEngine,
    models::{
        CreateHandoff, CreateProject, CreateSession, DecideProposals, GenerateTechnicalPlan,
        ProposalDecision, ReviseKnowledge, SendMessage,
    },
    service::{self, AppState},
};
use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn credits_v2_crosses_the_full_context_control_plane() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@127.0.0.1:54322/postgres".into());
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    let state = AppState {
        pool: pool.clone(),
        engine: Arc::new(DeterministicEngine),
        workspace_id: "10000000-0000-0000-0000-000000000001".parse()?,
        actor_id: "00000000-0000-0000-0000-000000000001".parse()?,
        agent_mode: "deterministic",
    };
    let project = service::create_project(
        &state,
        CreateProject {
            name: format!("Credits v2 test {}", Uuid::new_v4()),
            objective: "Des crédits achetés durables et sans expiration.".into(),
        },
    )
    .await?;
    let product_session = service::create_session(
        &state,
        project.public_id,
        CreateSession {
            node_key: "product".into(),
            title: Some("Cadrage test".into()),
        },
    )
    .await?;
    let product_session_id = product_session.session.public_id;
    let product_turn = service::send_message(
        &state,
        product_session_id,
        SendMessage {
            content: "Les crédits achetés n'expirent jamais.".into(),
        },
    )
    .await?;
    assert_eq!(product_turn.proposals.len(), 3);
    assert!(
        product_turn
            .proposals
            .iter()
            .all(|item| item.status == "proposed")
    );
    let product_proposal_ids = product_turn
        .proposals
        .iter()
        .map(|item| item.public_id)
        .collect::<Vec<_>>();
    let product_commit = service::decide_proposals(
        &state,
        product_session_id,
        DecideProposals {
            proposal_ids: product_proposal_ids.clone(),
            decision: ProposalDecision::Confirm,
        },
    )
    .await?;
    assert_eq!(product_commit.confirmed.len(), 3);
    assert_eq!(product_commit.graph_version, 1);
    let replay = service::decide_proposals(
        &state,
        product_session_id,
        DecideProposals {
            proposal_ids: product_proposal_ids,
            decision: ProposalDecision::Confirm,
        },
    )
    .await?;
    assert_eq!(replay.confirmed.len(), 3);
    assert_eq!(replay.graph_version, 1);

    let gate = service::evaluate_product_gate(&state, project.public_id).await?;
    assert_eq!(gate.status, "passed");
    assert!(gate.missing.is_empty());
    let brief = service::generate_feature_brief(&state, project.public_id).await?;
    assert_eq!(brief.deliverable_type, "feature-brief");
    assert_eq!(brief.status, "committed");

    let handoff = service::create_handoff(
        &state,
        project.public_id,
        CreateHandoff {
            source_session_id: product_session_id,
        },
    )
    .await?;
    assert_eq!(handoff.status, "completed");
    assert!(handoff.context_pack.get("provenance").is_some());
    let tech_turn = service::send_message(
        &state,
        handoff.target_session_public_id,
        SendMessage {
            content: "Purger les crédits non consommés après 90 jours.".into(),
        },
    )
    .await?;
    let tech_commit = service::decide_proposals(
        &state,
        handoff.target_session_public_id,
        DecideProposals {
            proposal_ids: tech_turn
                .proposals
                .iter()
                .filter(|item| item.status == "proposed")
                .map(|item| item.public_id)
                .collect(),
            decision: ProposalDecision::Confirm,
        },
    )
    .await?;
    assert_eq!(tech_commit.insight_ids.len(), 1);
    let edge_count: i64 = sqlx::query_scalar(
        "select count(*) from app.edges e
         join app.projects p on p.id = e.project_id
         where p.public_id = $1 and e.edge_type = 'derived_from'",
    )
    .bind(project.public_id)
    .fetch_one(&pool)
    .await?;
    assert!(edge_count >= 1);
    let contradiction = service::insight_detail(&state, tech_commit.insight_ids[0]).await?;
    assert_eq!(contradiction.insight.severity, "blocking");
    assert_eq!(contradiction.sources.len(), 2);

    let plan = service::generate_technical_plan(
        &state,
        project.public_id,
        GenerateTechnicalPlan {
            session_id: handoff.target_session_public_id,
        },
    )
    .await?;
    assert_eq!(plan.deliverable_type, "technical-delivery-plan");
    let coverage = service::coverage(&state, project.public_id).await?;
    assert!(coverage.total > 0);
    assert_eq!(
        coverage.total,
        coverage.covered + coverage.partial + coverage.missing
    );
    assert!(
        coverage
            .items
            .iter()
            .all(|item| item.evidence_public_id.is_some())
    );

    let revision = service::revise_knowledge(
        &state,
        tech_commit.confirmed[0],
        ReviseKnowledge {
            statement: "Le ledger est conservé sans purge automatique.".into(),
            title: None,
            rationale: Some("Alignement avec la règle Produit.".into()),
        },
    )
    .await?;
    assert_eq!(revision.version_number, 2);
    let resolved = service::insight_detail(&state, tech_commit.insight_ids[0]).await?;
    assert_eq!(resolved.insight.status, "resolved");

    let resumed = service::snapshot(&state, project.public_id).await?;
    assert_eq!(resumed.nodes.len(), 2);
    assert_eq!(resumed.sessions.len(), 2);
    assert!(resumed.project.graph_version >= 3);

    sqlx::query("update app.projects set status = 'archived' where public_id = $1")
        .bind(project.public_id)
        .execute(&pool)
        .await?;
    Ok(())
}
