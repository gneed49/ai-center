use std::sync::Arc;

use ai_center_server::{
    agent::DeterministicEngine,
    models::{
        CompileContextPack, CreateHandoff, CreateProject, CreateSession, DecideProposals,
        GenerateTechnicalPlan, InsightResolutionMutation, ProposalDecision, ResolveInsight,
        SendMessage,
    },
    outbox::OutboxPolicy,
    service::{self, AppState},
    steward::{StewardDrainSupervisor, StewardSupervisorPolicy},
};
use anyhow::{Context, Result, ensure};
use sqlx::{PgPool, Postgres, Transaction, postgres::PgPoolOptions};
use uuid::Uuid;

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn arbitrary_project_crosses_the_full_context_control_plane() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@127.0.0.1:54322/postgres".into());
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    if let Ok(expected_role) = std::env::var("AI_CENTER_EXPECT_DATABASE_ROLE") {
        let current_role: String = sqlx::query_scalar("select current_user")
            .fetch_one(&pool)
            .await?;
        ensure!(
            current_role == expected_role,
            "integration flow connected as {current_role}, expected {expected_role}"
        );
    }
    let workspace_id = "10000000-0000-0000-0000-000000000001".parse()?;
    let actor_id = "00000000-0000-0000-0000-000000000001".parse()?;
    let (workspace_internal_id, workspace_role): (i64, String) =
        sqlx::query_as("select workspace_id, role from app.authorize_workspace_member($1, $2)")
            .bind(workspace_id)
            .bind(actor_id)
            .fetch_one(&pool)
            .await?;
    let state = AppState {
        pool: pool.clone(),
        engine: Arc::new(DeterministicEngine),
        workspace_id,
        workspace_internal_id: Some(workspace_internal_id),
        workspace_role: workspace_role.clone(),
        actor_id,
        agent_mode: "deterministic",
        steward_trigger: None,
    };
    let project = service::create_project(
        &state,
        CreateProject {
            name: format!("Context proof test {}", Uuid::new_v4()),
            objective: "Des décisions validées, durables et traçables.".into(),
        },
    )
    .await
    .context("create project")?;
    let product_session = service::create_session(
        &state,
        project.public_id,
        CreateSession {
            node_key: "product".into(),
            title: Some("Cadrage test".into()),
        },
    )
    .await
    .context("create Product session")?;
    let product_session_id = product_session.session.public_id;
    let product_turn = service::send_message(
        &state,
        product_session_id,
        SendMessage {
            content: "Les décisions validées restent consultables sans expiration.".into(),
            client_message_id: Uuid::new_v4(),
        },
    )
    .await
    .context("send Product message")?;
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

    let context_pack = service::compile_context_pack(
        &state,
        project.public_id,
        CompileContextPack {
            source_session_id: product_session_id,
            task_kind: "technical-delivery-plan".into(),
            token_budget: Some(12_000),
        },
    )
    .await?;
    let handoff = service::create_handoff(
        &state,
        project.public_id,
        CreateHandoff {
            source_session_id: product_session_id,
            context_pack_id: context_pack.public_id,
        },
    )
    .await?;
    assert_eq!(handoff.status, "completed");
    assert!(handoff.context_pack.content.get("provenance").is_some());
    let tech_turn = service::send_message(
        &state,
        handoff.target_session_public_id,
        SendMessage {
            content: "Purger automatiquement les décisions après 90 jours.".into(),
            client_message_id: Uuid::new_v4(),
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
    assert!(
        !tech_commit.insight_ids.is_empty(),
        "the conflicting technical rule must yield at least one insight"
    );
    let mut verification_tx =
        begin_scoped_test_transaction(&pool, actor_id, workspace_internal_id, &workspace_role)
            .await?;
    let edge_count: i64 = sqlx::query_scalar(
        "select count(*) from app.edges e
         join app.projects p on p.id = e.project_id
         where p.public_id = $1 and e.edge_type = 'derived_from'",
    )
    .bind(project.public_id)
    .fetch_one(&mut *verification_tx)
    .await?;
    let (pending_events, processing_events, processed_events): (i64, i64, i64) = sqlx::query_as(
        "select
               count(*) filter (where event.status = 'pending')::bigint,
               count(*) filter (where event.status = 'processing')::bigint,
               count(*) filter (where event.status = 'processed')::bigint
             from app.domain_events event
             join app.projects project on project.id = event.project_id
             where project.public_id = $1
               and event.event_type in ('knowledge.committed', 'knowledge.revised')",
    )
    .bind(project.public_id)
    .fetch_one(&mut *verification_tx)
    .await?;
    verification_tx.commit().await?;
    assert!(edge_count >= 1);
    assert_eq!(
        pending_events, 0,
        "a successful Steward drain must ack every event"
    );
    assert_eq!(
        processing_events, 0,
        "no Steward lease may survive a successful deterministic drain"
    );
    let expected_processed_events =
        i64::try_from(product_commit.confirmed.len() + tech_commit.confirmed.len())?;
    assert!(
        processed_events >= expected_processed_events,
        "every confirmed knowledge item must cross claim/process/ack"
    );
    let contradiction = service::insight_detail(&state, tech_commit.insight_ids[0]).await?;
    assert_eq!(contradiction.insight.severity, "blocking");
    assert_eq!(contradiction.sources.len(), 2);
    let (conflicting_tech_knowledge_id, conflicting_tech_version_id) = contradiction
        .sources
        .iter()
        .filter_map(|source| Some((source.knowledge_public_id?, source.version_public_id?)))
        .find(|(knowledge_id, _)| tech_commit.confirmed.contains(knowledge_id))
        .context("blocking insight must retain its confirmed Tech source")?;

    let stale_plan_error = service::generate_technical_plan(
        &state,
        project.public_id,
        GenerateTechnicalPlan {
            session_id: handoff.target_session_public_id,
        },
    )
    .await
    .expect_err("a stale Tech session must not generate a plan");
    assert!(stale_plan_error.to_string().contains("stale"));

    let resolved = service::resolve_insight(
        &state,
        project.public_id,
        tech_commit.insight_ids[0],
        ResolveInsight {
            expected_graph_version: contradiction.insight.project_graph_version,
            justification: "Alignement avec la règle Produit.".into(),
            mutations: vec![InsightResolutionMutation::ReviseKnowledge {
                knowledge_public_id: conflicting_tech_knowledge_id,
                expected_version_public_id: conflicting_tech_version_id,
                statement: "Les décisions restent consultables sans purge automatique.".into(),
                title: None,
                rationale: Some("Alignement avec la règle Produit.".into()),
            }],
        },
    )
    .await?;
    assert_eq!(resolved.insight.status, "resolved");

    let stale_pack = service::context_pack(&state, context_pack.public_id).await?;
    assert_eq!(stale_pack.status, "stale");
    let recompiled_pack = service::compile_context_pack(
        &state,
        project.public_id,
        CompileContextPack {
            source_session_id: product_session_id,
            task_kind: "technical-delivery-plan".into(),
            token_budget: Some(12_000),
        },
    )
    .await?;
    assert_eq!(recompiled_pack.status, "current");
    assert!(recompiled_pack.source_graph_version > context_pack.source_graph_version);
    let recompiled_handoff = service::create_handoff(
        &state,
        project.public_id,
        CreateHandoff {
            source_session_id: product_session_id,
            context_pack_id: recompiled_pack.public_id,
        },
    )
    .await?;
    let plan = service::generate_technical_plan(
        &state,
        project.public_id,
        GenerateTechnicalPlan {
            session_id: recompiled_handoff.target_session_public_id,
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
            .all(|item| item.evidence_public_id.is_none() && item.status == "missing")
    );

    let resumed = service::snapshot(&state, project.public_id).await?;
    assert_eq!(resumed.nodes.len(), 2);
    assert_eq!(resumed.sessions.len(), 3);
    assert!(resumed.project.graph_version >= 3);

    let mut cleanup_tx =
        begin_scoped_test_transaction(&pool, actor_id, workspace_internal_id, &workspace_role)
            .await?;
    let unfinished_steward_events: i64 = sqlx::query_scalar(
        "select count(*)
         from app.domain_events event
         join app.projects project on project.id = event.project_id
         where project.public_id = $1
           and event.event_type in ('knowledge.committed', 'knowledge.revised')
           and event.status in ('pending', 'processing')",
    )
    .bind(project.public_id)
    .fetch_one(&mut *cleanup_tx)
    .await?;
    assert_eq!(
        unfinished_steward_events, 0,
        "the insight-resolution revision must also finish through the outbox"
    );
    sqlx::query(
        "update app.projects set status = 'archived'
         where public_id = $1 and workspace_id = $2",
    )
    .bind(project.public_id)
    .bind(workspace_internal_id)
    .execute(&mut *cleanup_tx)
    .await?;
    cleanup_tx.commit().await?;

    // Exercise the exact post-commit dispatch used in OpenAI mode while
    // keeping the deterministic engine as the provider double. The API result
    // must not wait for Steward, and the supervisor must eventually finish the
    // durable claim/process/ack lifecycle.
    let mut async_state = AppState {
        pool: pool.clone(),
        engine: Arc::new(DeterministicEngine),
        workspace_id,
        workspace_internal_id: Some(workspace_internal_id),
        workspace_role: workspace_role.clone(),
        actor_id,
        agent_mode: "openai",
        steward_trigger: None,
    };
    let (trigger, supervisor) = StewardDrainSupervisor::start(async_state.clone())?;
    async_state.steward_trigger = Some(trigger);
    let async_project = service::create_project(
        &async_state,
        CreateProject {
            name: format!("Async Steward test {}", Uuid::new_v4()),
            objective: "Prouver le drain Steward supervisé après commit.".into(),
        },
    )
    .await?;
    let async_session = service::create_session(
        &async_state,
        async_project.public_id,
        CreateSession {
            node_key: "product".into(),
            title: Some("Contexte asynchrone".into()),
        },
    )
    .await?;
    let async_turn = service::send_message(
        &async_state,
        async_session.session.public_id,
        SendMessage {
            content: "Les décisions validées restent consultables sans expiration.".into(),
            client_message_id: Uuid::new_v4(),
        },
    )
    .await?;
    let async_commit = service::decide_proposals(
        &async_state,
        async_session.session.public_id,
        DecideProposals {
            proposal_ids: async_turn
                .proposals
                .iter()
                .map(|proposal| proposal.public_id)
                .collect(),
            decision: ProposalDecision::Confirm,
        },
    )
    .await?;
    assert!(
        async_commit.insight_ids.is_empty(),
        "OpenAI mode must return after enqueueing rather than waiting for Steward"
    );

    let mut async_counts = (0_i64, 0_i64, 0_i64);
    for _ in 0..100 {
        async_counts = steward_event_counts(
            &pool,
            actor_id,
            workspace_internal_id,
            &workspace_role,
            async_project.public_id,
        )
        .await?;
        if async_counts.0 == 0 && async_counts.1 == 0 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    assert_eq!(
        async_counts.0, 0,
        "the supervised drain must consume every available pending event"
    );
    assert_eq!(
        async_counts.1, 0,
        "the supervised drain must acknowledge every claimed event"
    );
    assert!(
        async_counts.2 >= i64::try_from(async_commit.confirmed.len())?,
        "the async lifecycle must persist every knowledge event as processed"
    );

    let mut async_cleanup_tx =
        begin_scoped_test_transaction(&pool, actor_id, workspace_internal_id, &workspace_role)
            .await?;
    sqlx::query(
        "update app.projects set status = 'archived'
         where public_id = $1 and workspace_id = $2",
    )
    .bind(async_project.public_id)
    .bind(workspace_internal_id)
    .execute(&mut *async_cleanup_tx)
    .await?;
    async_cleanup_tx.commit().await?;
    drop(async_state);
    supervisor.shutdown().await;

    // Simulate a process loss between the durable context commit and its
    // in-memory trigger. No request is made after the new supervisor starts:
    // only the private periodic scanner can discover and recover this work.
    let recovery_state = AppState {
        pool: pool.clone(),
        engine: Arc::new(DeterministicEngine),
        workspace_id,
        workspace_internal_id: Some(workspace_internal_id),
        workspace_role: workspace_role.clone(),
        actor_id,
        agent_mode: "openai",
        steward_trigger: None,
    };
    let recovery_project = service::create_project(
        &recovery_state,
        CreateProject {
            name: format!("Restart recovery test {}", Uuid::new_v4()),
            objective: "Reprendre un événement durable sans requête cliente.".into(),
        },
    )
    .await?;
    let recovery_session = service::create_session(
        &recovery_state,
        recovery_project.public_id,
        CreateSession {
            node_key: "product".into(),
            title: Some("Contexte avant restart".into()),
        },
    )
    .await?;
    let recovery_turn = service::send_message(
        &recovery_state,
        recovery_session.session.public_id,
        SendMessage {
            content: "Les décisions validées restent consultables sans expiration.".into(),
            client_message_id: Uuid::new_v4(),
        },
    )
    .await?;
    let recovery_commit = service::decide_proposals(
        &recovery_state,
        recovery_session.session.public_id,
        DecideProposals {
            proposal_ids: recovery_turn
                .proposals
                .iter()
                .map(|proposal| proposal.public_id)
                .collect(),
            decision: ProposalDecision::Confirm,
        },
    )
    .await?;
    let mut simulated_crash_tx =
        begin_scoped_test_transaction(&pool, actor_id, workspace_internal_id, &workspace_role)
            .await?;
    let expired_claim_public_id: Uuid = sqlx::query_scalar(
        "with target as (
           select event.id
           from app.domain_events event
           join app.projects project on project.id = event.project_id
           where project.public_id = $1
             and event.event_type in ('knowledge.committed', 'knowledge.revised')
             and event.status = 'pending'
           order by event.id
           limit 1
           for update of event
         )
         update app.domain_events event
         set status = 'processing',
             attempt_count = 1,
             locked_at = clock_timestamp() - interval '2 minutes',
             locked_until = clock_timestamp() - interval '1 minute',
             locked_by = 'simulated-crashed-worker'
         from target
         where event.id = target.id
         returning event.public_id",
    )
    .bind(recovery_project.public_id)
    .fetch_one(&mut *simulated_crash_tx)
    .await?;
    simulated_crash_tx.commit().await?;
    assert_ne!(expired_claim_public_id, Uuid::nil());

    let before_restart = steward_event_counts(
        &pool,
        actor_id,
        workspace_internal_id,
        &workspace_role,
        recovery_project.public_id,
    )
    .await?;
    assert!(
        before_restart.0 + before_restart.1 >= i64::try_from(recovery_commit.confirmed.len())?,
        "the simulated lost trigger and crashed worker must leave durable events"
    );
    assert_eq!(
        before_restart.1, 1,
        "one claimed event must retain the expired crashed-worker lease"
    );

    let (scanner_keepalive, recovery_supervisor) = StewardDrainSupervisor::start_with_policies(
        recovery_state.clone(),
        StewardSupervisorPolicy {
            scan_interval: std::time::Duration::from_secs(1),
            max_workspaces_per_scan: 8,
        },
        OutboxPolicy {
            lease_duration: std::time::Duration::from_secs(60),
            max_attempts: 5,
            base_backoff: std::time::Duration::from_millis(1),
            max_backoff: std::time::Duration::from_millis(1),
        },
    )?;
    let mut recovered_counts = before_restart;
    for _ in 0..160 {
        recovered_counts = steward_event_counts(
            &pool,
            actor_id,
            workspace_internal_id,
            &workspace_role,
            recovery_project.public_id,
        )
        .await?;
        if recovered_counts.0 == 0 && recovered_counts.1 == 0 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    assert_eq!(
        recovered_counts.0, 0,
        "restart recovery must consume pending Steward events without a request trigger"
    );
    assert_eq!(
        recovered_counts.1, 0,
        "restart recovery must leave no claimed event behind"
    );
    assert!(
        recovered_counts.2 >= i64::try_from(recovery_commit.confirmed.len())?,
        "restart recovery must durably acknowledge every context event"
    );
    drop(scanner_keepalive);
    recovery_supervisor.shutdown().await;

    let mut recovery_cleanup_tx =
        begin_scoped_test_transaction(&pool, actor_id, workspace_internal_id, &workspace_role)
            .await?;
    sqlx::query(
        "update app.projects set status = 'archived'
         where public_id = $1 and workspace_id = $2",
    )
    .bind(recovery_project.public_id)
    .bind(workspace_internal_id)
    .execute(&mut *recovery_cleanup_tx)
    .await?;
    recovery_cleanup_tx.commit().await?;
    Ok(())
}

async fn steward_event_counts(
    pool: &PgPool,
    actor_id: Uuid,
    workspace_id: i64,
    workspace_role: &str,
    project_public_id: Uuid,
) -> Result<(i64, i64, i64)> {
    let mut tx =
        begin_scoped_test_transaction(pool, actor_id, workspace_id, workspace_role).await?;
    let counts = sqlx::query_as(
        "select
           count(*) filter (where event.status = 'pending')::bigint,
           count(*) filter (where event.status = 'processing')::bigint,
           count(*) filter (where event.status = 'processed')::bigint
         from app.domain_events event
         join app.projects project on project.id = event.project_id
         where project.public_id = $1
           and event.event_type in ('knowledge.committed', 'knowledge.revised')",
    )
    .bind(project_public_id)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(counts)
}

async fn begin_scoped_test_transaction<'a>(
    pool: &'a PgPool,
    actor_id: Uuid,
    workspace_id: i64,
    workspace_role: &str,
) -> Result<Transaction<'a, Postgres>> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "select set_config('app.current_actor_id', $1, true),
                set_config('app.current_workspace_id', $2, true),
                set_config('app.current_workspace_role', $3, true)",
    )
    .bind(actor_id.to_string())
    .bind(workspace_id.to_string())
    .bind(workspace_role)
    .execute(&mut *tx)
    .await?;
    Ok(tx)
}
