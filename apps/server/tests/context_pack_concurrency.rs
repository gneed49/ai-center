use std::{sync::Arc, time::Duration};

use ai_center_server::{
    agent::{
        AgentEngine, AgentInput, ContextSelectionDraft, ContextSelectionInput,
        CoverageEvaluationDraft, CoverageEvaluationInput, DeterministicEngine, EngineOutput,
        StewardInput, StewardOutput, TechnicalPlanDraft, TechnicalPlanInput,
    },
    error::{AppError, AppResult},
    models::{
        AgentTurn, CompileContextPack, CreateHandoff, CreateProject, CreateSession,
        DecideProposals, ProposalDecision, ReviseKnowledge, SendMessage,
    },
    service::{self, AppState},
};
use anyhow::{Context, Result, ensure};
use async_trait::async_trait;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::sync::Notify;
use uuid::Uuid;

struct BlockingRespondEngine {
    entered: Arc<Notify>,
    release: Arc<Notify>,
}

#[async_trait]
impl AgentEngine for BlockingRespondEngine {
    fn provider_name(&self) -> &'static str {
        "deterministic"
    }

    fn requested_model(&self) -> &'static str {
        "deterministic-test-double"
    }

    async fn respond(&self, input: AgentInput) -> AppResult<EngineOutput<AgentTurn>> {
        let output = DeterministicEngine.respond(input).await?;
        self.entered.notify_one();
        self.release.notified().await;
        Ok(output)
    }

    async fn select_context(
        &self,
        input: ContextSelectionInput,
    ) -> AppResult<EngineOutput<ContextSelectionDraft>> {
        DeterministicEngine.select_context(input).await
    }

    async fn generate_technical_plan(
        &self,
        input: TechnicalPlanInput,
    ) -> AppResult<EngineOutput<TechnicalPlanDraft>> {
        DeterministicEngine.generate_technical_plan(input).await
    }

    async fn evaluate_coverage(
        &self,
        input: CoverageEvaluationInput,
    ) -> AppResult<EngineOutput<CoverageEvaluationDraft>> {
        DeterministicEngine.evaluate_coverage(input).await
    }

    async fn analyze_contradictions(
        &self,
        input: StewardInput,
    ) -> AppResult<EngineOutput<StewardOutput>> {
        DeterministicEngine.analyze_contradictions(input).await
    }
}

struct ContextFixture {
    state: AppState,
    admin_pool: PgPool,
    project_public_id: Uuid,
    product_session_id: Uuid,
    pack_public_id: Uuid,
}

#[tokio::test]
async fn feature_brief_publication_serializes_with_source_revision() -> Result<()> {
    let fixture = prepare_context_fixture().await?;
    let snapshot = service::snapshot(&fixture.state, fixture.project_public_id).await?;
    let requirement = snapshot
        .knowledge
        .iter()
        .find(|entry| entry.entry_type == "requirement")
        .context("requirement fixture")?;
    // Block exactly the deliverable insert, after its source reads, on the
    // contract FK. Gate evaluation does not touch this row.
    let mut contract_lock = fixture.admin_pool.begin().await?;
    sqlx::query(
        "select id from app.deliverable_contracts where contract_key='feature-brief' for update",
    )
    .fetch_all(&mut *contract_lock)
    .await?;
    let blocker_xid: String = sqlx::query_scalar("select pg_current_xact_id()::text")
        .fetch_one(&mut *contract_lock)
        .await?;
    let brief_state = fixture.state.clone();
    let project_id = fixture.project_public_id;
    let brief_task =
        tokio::spawn(
            async move { service::generate_feature_brief(&brief_state, project_id).await },
        );
    let brief_pid = wait_for_transaction_waiter(&fixture.admin_pool, &blocker_xid)
        .await?
        .context("brief did not reach publication")?;
    let publication_transaction_id = transaction_id_for_pid(&fixture.admin_pool, brief_pid)
        .await?
        .context("brief transaction missing")?;
    let revision_state = fixture.state.clone();
    let requirement_id = requirement.public_id;
    let revision = ReviseKnowledge {
        title: Some(requirement.title.clone()),
        statement: format!("{} Une révision concurrente.", requirement.statement),
        rationale: Some(requirement.rationale.clone()),
    };
    let revision_task = tokio::spawn(async move {
        service::revise_knowledge_for_project(&revision_state, project_id, requirement_id, revision)
            .await
    });
    let revision_waiter =
        wait_for_transaction_waiter(&fixture.admin_pool, &publication_transaction_id).await?;
    contract_lock.commit().await?;
    let brief = brief_task.await??;
    revision_task.await??;
    ensure!(
        revision_waiter.is_some(),
        "revision must wait for the brief's source/commit transaction"
    );
    let after = service::snapshot(&fixture.state, project_id).await?;
    let committed = after
        .deliverables
        .iter()
        .find(|item| item.public_id == brief.public_id)
        .context("brief disappeared")?;
    ensure!(
        committed.status == "stale",
        "a later source revision must invalidate the just-published brief"
    );
    Ok(())
}

#[tokio::test]
async fn tech_response_is_rolled_back_when_its_pack_is_superseded_in_flight() -> Result<()> {
    let fixture = prepare_context_fixture().await?;
    let handoff = service::create_handoff(
        &fixture.state,
        fixture.project_public_id,
        CreateHandoff {
            source_session_id: fixture.product_session_id,
            context_pack_id: fixture.pack_public_id,
        },
    )
    .await?;

    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let mut blocked_state = fixture.state.clone();
    blocked_state.engine = Arc::new(BlockingRespondEngine {
        entered: Arc::clone(&entered),
        release: Arc::clone(&release),
    });
    let tech_session_id = handoff.target_session_public_id;
    let send_task = tokio::spawn(async move {
        service::send_message(
            &blocked_state,
            tech_session_id,
            SendMessage {
                content: "Produire une réponse exclusivement depuis ce ContextPack.".into(),
                client_message_id: Uuid::new_v4(),
            },
        )
        .await
    });

    tokio::time::timeout(Duration::from_secs(5), entered.notified())
        .await
        .context("the provider seam was never reached")?;
    let recompiled = service::compile_context_pack(
        &fixture.state,
        fixture.project_public_id,
        CompileContextPack {
            source_session_id: fixture.product_session_id,
            task_kind: "technical-delivery-plan".into(),
            token_budget: Some(12_000),
        },
    )
    .await?;
    ensure!(recompiled.public_id != fixture.pack_public_id);
    release.notify_one();

    let send_result = send_task.await?;
    ensure!(
        matches!(send_result, Err(AppError::Conflict(ref message)) if message.contains("stale")),
        "an in-flight response from a superseded ContextPack must be rejected"
    );
    let session = service::session_view(&fixture.state, tech_session_id).await?;
    ensure!(
        session.messages.len() == 1 && session.messages[0].role == "user",
        "the preserved user input must be the only message after stale-output rollback"
    );
    ensure!(
        session.proposals.is_empty(),
        "a stale provider result must not persist mutation proposals"
    );
    Ok(())
}

#[tokio::test]
async fn handoff_serializes_with_recompile_and_rejects_the_superseded_pack() -> Result<()> {
    let fixture = prepare_context_fixture().await?;
    // Block the first handoff mutation on its Tech-node foreign key. This
    // leaves the transaction precisely between validating the pack and
    // inserting the Tech session, which is the stale-pack race window.
    let mut node_lock = fixture.admin_pool.begin().await?;
    let _: i64 = sqlx::query_scalar(
        "select node.id
         from app.context_nodes node
         join app.projects project on project.id = node.project_id
         where project.public_id = $1 and node.node_key = 'tech'
         for update of node",
    )
    .bind(fixture.project_public_id)
    .fetch_one(&mut *node_lock)
    .await?;
    let blocker_xid: String = sqlx::query_scalar("select pg_current_xact_id()::text")
        .fetch_one(&mut *node_lock)
        .await?;

    let handoff_state = fixture.state.clone();
    let project_public_id = fixture.project_public_id;
    let product_session_id = fixture.product_session_id;
    let pack_public_id = fixture.pack_public_id;
    let handoff_task = tokio::spawn(async move {
        service::create_handoff(
            &handoff_state,
            project_public_id,
            CreateHandoff {
                source_session_id: product_session_id,
                context_pack_id: pack_public_id,
            },
        )
        .await
    });
    let blocked_backend_pid = wait_for_transaction_waiter(&fixture.admin_pool, &blocker_xid)
        .await?
        .context("handoff never reached the Tech-session insertion boundary")?;
    let domain_transaction_id = transaction_id_for_pid(&fixture.admin_pool, blocked_backend_pid)
        .await?
        .context("handoff transaction has no visible transaction id")?;

    let recompile_state = fixture.state.clone();
    let product_session_id = fixture.product_session_id;
    let recompile_task = tokio::spawn(async move {
        service::compile_context_pack(
            &recompile_state,
            project_public_id,
            CompileContextPack {
                source_session_id: product_session_id,
                task_kind: "technical-delivery-plan".into(),
                token_budget: Some(12_000),
            },
        )
        .await
    });
    let recompile_waiter =
        wait_for_transaction_waiter(&fixture.admin_pool, &domain_transaction_id).await?;

    node_lock.commit().await?;
    let handoff = handoff_task.await??;
    let recompiled = recompile_task.await??;
    ensure!(
        recompile_waiter.is_some(),
        "recompile must wait for the handoff's project/pack lock before superseding it"
    );
    ensure!(handoff.context_pack_public_id == fixture.pack_public_id);
    ensure!(recompiled.public_id != fixture.pack_public_id);

    let old_pack = service::context_pack(&fixture.state, fixture.pack_public_id).await?;
    ensure!(old_pack.status == "superseded");
    let stale_handoff = service::create_handoff(
        &fixture.state,
        fixture.project_public_id,
        CreateHandoff {
            source_session_id: fixture.product_session_id,
            context_pack_id: fixture.pack_public_id,
        },
    )
    .await;
    ensure!(
        matches!(stale_handoff, Err(AppError::Conflict(ref message)) if message.contains("stale")),
        "a superseded pack must not create or replay a handoff"
    );
    Ok(())
}

async fn prepare_context_fixture() -> Result<ContextFixture> {
    let (pool, admin_pool) = test_database_pools().await?;
    let workspace_id = "10000000-0000-0000-0000-000000000001".parse()?;
    let actor_id = "00000000-0000-0000-0000-000000000001".parse()?;
    let (workspace_internal_id, workspace_role): (i64, String) =
        sqlx::query_as("select workspace_id, role from app.authorize_workspace_member($1, $2)")
            .bind(workspace_id)
            .bind(actor_id)
            .fetch_one(&pool)
            .await?;
    let state = AppState {
        pool,
        engine: Arc::new(DeterministicEngine),
        workspace_id,
        workspace_internal_id: Some(workspace_internal_id),
        workspace_role,
        actor_id,
        agent_mode: "deterministic",
        steward_trigger: None,
    };
    let project = service::create_project(
        &state,
        CreateProject {
            name: format!("ContextPack concurrency {}", Uuid::new_v4()),
            objective: "Des décisions validées, durables et traçables.".into(),
        },
    )
    .await?;
    let product_session = service::create_session(
        &state,
        project.public_id,
        CreateSession {
            node_key: "product".into(),
            title: Some("Cadrage concurrent".into()),
        },
    )
    .await?;
    let product_session_id = product_session.session.public_id;
    let turn = service::send_message(
        &state,
        product_session_id,
        SendMessage {
            content: "Les décisions validées restent consultables sans expiration.".into(),
            client_message_id: Uuid::new_v4(),
        },
    )
    .await?;
    service::decide_proposals(
        &state,
        product_session_id,
        DecideProposals {
            proposal_ids: turn
                .proposals
                .iter()
                .map(|proposal| proposal.public_id)
                .collect(),
            decision: ProposalDecision::Confirm,
        },
    )
    .await?;
    let pack = service::compile_context_pack(
        &state,
        project.public_id,
        CompileContextPack {
            source_session_id: product_session_id,
            task_kind: "technical-delivery-plan".into(),
            token_budget: Some(12_000),
        },
    )
    .await?;
    Ok(ContextFixture {
        state,
        admin_pool,
        project_public_id: project.public_id,
        product_session_id,
        pack_public_id: pack.public_id,
    })
}

async fn test_database_pools() -> Result<(PgPool, PgPool)> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@127.0.0.1:54322/postgres".into());
    let expected_runtime_role = std::env::var("AI_CENTER_EXPECT_DATABASE_ROLE").ok();
    let admin_database_url = match std::env::var("AI_CENTER_ADMIN_DATABASE_URL") {
        Ok(value) if !value.trim().is_empty() => value,
        Ok(_) | Err(_) if expected_runtime_role.is_some() => anyhow::bail!(
            "AI_CENTER_ADMIN_DATABASE_URL is required when AI_CENTER_EXPECT_DATABASE_ROLE is set"
        ),
        Ok(_) | Err(_) => database_url.clone(),
    };
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&database_url)
        .await?;
    if let Some(expected_role) = expected_runtime_role.as_deref() {
        let current_role: String = sqlx::query_scalar("select current_user")
            .fetch_one(&pool)
            .await?;
        ensure!(
            current_role == expected_role,
            "integration flow connected as {current_role}, expected {expected_role}"
        );
    }
    // This second pool is a test-control seam only: it may hold a row lock and
    // observe pg_locks, but every AI Center service call continues to use the
    // runtime pool above and therefore remains subject to runtime grants/RLS.
    let admin_pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&admin_database_url)
        .await?;
    let admin_role: String = sqlx::query_scalar("select current_user")
        .fetch_one(&admin_pool)
        .await?;
    ensure!(
        admin_role != "ai_center_runtime",
        "AI_CENTER_ADMIN_DATABASE_URL must not resolve to ai_center_runtime"
    );
    Ok((pool, admin_pool))
}

async fn wait_for_transaction_waiter(pool: &PgPool, transaction_id: &str) -> Result<Option<i32>> {
    let waiter = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let waiter: Option<i32> = sqlx::query_scalar(
                "select pid from pg_locks
                 where locktype = 'transactionid'
                   and transactionid::text = $1
                   and not granted
                 order by pid
                 limit 1",
            )
            .bind(transaction_id)
            .fetch_optional(pool)
            .await?;
            if waiter.is_some() {
                return Ok::<Option<i32>, sqlx::Error>(waiter);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    match waiter {
        Ok(result) => Ok(result?),
        Err(_) => Ok(None),
    }
}

async fn transaction_id_for_pid(pool: &PgPool, pid: i32) -> Result<Option<String>> {
    Ok(sqlx::query_scalar(
        "select transactionid::text from pg_locks
         where pid = $1 and locktype = 'transactionid' and granted
         order by transactionid
         limit 1",
    )
    .bind(pid)
    .fetch_optional(pool)
    .await?)
}
