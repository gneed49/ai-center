use std::sync::Arc;

use ai_center_server::{
    agent::DeterministicEngine,
    error::{AppError, ProviderError, ProviderErrorClass},
    idempotency::{self, BeginOutcome, BeginRequest, FailureDisposition, IdempotencyLease},
    models::{
        CompileContextPack, CreateHandoff, CreateProject, CreateSession, DecideProposals,
        GenerateTechnicalPlan, ProposalDecision, ReviseKnowledge, SendMessage,
    },
    service::{self, AppState},
};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use serde_json::json;
use sqlx::{Postgres, Transaction, postgres::PgPoolOptions};
use uuid::Uuid;

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn command_result_is_atomic_with_every_representative_domain_mutation() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@127.0.0.1:54322/postgres".into());
    let pool = PgPoolOptions::new()
        .max_connections(8)
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

    // Crash injection: claim is durable, but both the provisional project and
    // the provisional result are rolled back together at the final boundary.
    let project_name = format!("Atomic command {}", Uuid::new_v4());
    let project_input = CreateProject {
        name: project_name.clone(),
        objective: "Prouver un contexte durable sans double mutation.".into(),
    };
    let project_hash = idempotency::hash_request(&project_input)?;
    let project_key = Uuid::new_v4();
    let first_lease =
        expect_new(claim(&state, None, "project.create", project_key, &project_hash).await?)?;
    let mut crashed_tx = begin_scoped_transaction(&state).await?;
    let template_id: i64 = sqlx::query_scalar(
        "select id from app.project_templates where template_key = 'software-product-delivery'",
    )
    .fetch_one(&mut *crashed_tx)
    .await?;
    sqlx::query(
        "insert into app.projects (workspace_id, template_id, name, objective, created_by_actor_id)
         values ($1,$2,$3,$4,$5)",
    )
    .bind(workspace_internal_id)
    .bind(template_id)
    .bind(&project_name)
    .bind(&project_input.objective)
    .bind(actor_id)
    .execute(&mut *crashed_tx)
    .await?;
    idempotency::complete(
        &mut crashed_tx,
        &first_lease,
        200,
        json!({"must_not_survive": true}),
    )
    .await?;
    crashed_tx.rollback().await?;

    let mut verify_tx = begin_scoped_transaction(&state).await?;
    let rolled_back_projects: i64 =
        sqlx::query_scalar("select count(*) from app.projects where name = $1")
            .bind(&project_name)
            .fetch_one(&mut *verify_tx)
            .await?;
    let processing_status: String =
        sqlx::query_scalar("select status from app.idempotency_records where public_id = $1")
            .bind(first_lease.record_public_id)
            .fetch_one(&mut *verify_tx)
            .await?;
    ensure!(rolled_back_projects == 0, "crashed project must roll back");
    ensure!(
        processing_status == "processing",
        "crashed result must not become replayable"
    );
    sqlx::query(
        "update app.idempotency_records set locked_until = now() - interval '1 second'
         where public_id = $1",
    )
    .bind(first_lease.record_public_id)
    .execute(&mut *verify_tx)
    .await?;
    verify_tx.commit().await?;

    let reclaimed =
        expect_new(claim(&state, None, "project.create", project_key, &project_hash).await?)?;
    let project = service::create_project_idempotent(&state, project_input, &reclaimed)
        .await
        .context("create project after reclaim")?;
    assert_replay(
        claim(&state, None, "project.create", project_key, &project_hash).await?,
        &project,
    )?;
    let conflicting_hash = idempotency::hash_request(&CreateProject {
        name: project_name.clone(),
        objective: "A different body".into(),
    })?;
    ensure!(
        matches!(
            claim(
                &state,
                None,
                "project.create",
                project_key,
                &conflicting_hash,
            )
            .await,
            Err(AppError::Conflict(_))
        ),
        "same key with a different body must be a conflict"
    );

    let project_internal_id: i64 = scoped_scalar(
        &state,
        "select id from app.projects where public_id = $1",
        project.public_id,
    )
    .await?;
    let project_count: i64 = scoped_scalar(
        &state,
        "select count(*) from app.projects where name = $1",
        &project_name,
    )
    .await?;
    ensure!(project_count == 1, "reclaim created a duplicate project");

    let session_input = CreateSession {
        node_key: "product".into(),
        title: Some("Atomic Product session".into()),
    };
    let session_hash = idempotency::hash_request(&session_input)?;
    let session_key = Uuid::new_v4();
    let session_lease = expect_new(
        claim(
            &state,
            Some(project_internal_id),
            "session.create",
            session_key,
            &session_hash,
        )
        .await?,
    )?;
    let product_session = service::create_session_idempotent(
        &state,
        project.public_id,
        session_input,
        &session_lease,
    )
    .await?;
    assert_replay(
        claim(
            &state,
            Some(project_internal_id),
            "session.create",
            session_key,
            &session_hash,
        )
        .await?,
        &product_session,
    )?;

    let turn = service::send_message(
        &state,
        product_session.session.public_id,
        SendMessage {
            content: "Les décisions confirmées restent disponibles, traçables et testables.".into(),
            client_message_id: Uuid::new_v4(),
        },
    )
    .await?;
    service::decide_proposals(
        &state,
        product_session.session.public_id,
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

    let gate_key = Uuid::new_v4();
    let gate_hash = idempotency::hash_request(&"product-ready")?;
    let gate_lease = expect_new(
        claim(
            &state,
            Some(project_internal_id),
            "gate.product_ready.evaluate",
            gate_key,
            &gate_hash,
        )
        .await?,
    )?;
    let gate =
        service::evaluate_product_gate_idempotent(&state, project.public_id, &gate_lease).await?;
    ensure!(gate.status == "passed", "product gate did not pass");
    assert_replay(
        claim(
            &state,
            Some(project_internal_id),
            "gate.product_ready.evaluate",
            gate_key,
            &gate_hash,
        )
        .await?,
        &gate,
    )?;

    let brief_key = Uuid::new_v4();
    let brief_hash = idempotency::hash_request(&"feature-brief")?;
    let brief_lease = expect_new(
        claim(
            &state,
            Some(project_internal_id),
            "deliverable.feature_brief.generate",
            brief_key,
            &brief_hash,
        )
        .await?,
    )?;
    let brief =
        service::generate_feature_brief_idempotent(&state, project.public_id, &brief_lease).await?;
    assert_replay(
        claim(
            &state,
            Some(project_internal_id),
            "deliverable.feature_brief.generate",
            brief_key,
            &brief_hash,
        )
        .await?,
        &brief,
    )?;

    let pack_input = CompileContextPack {
        source_session_id: product_session.session.public_id,
        task_kind: "technical-delivery-plan".into(),
        token_budget: Some(12_000),
    };
    let pack_hash = idempotency::hash_request(&pack_input)?;
    let pack_key = Uuid::new_v4();
    let pack_lease = expect_new(
        claim(
            &state,
            Some(project_internal_id),
            "context_pack.compile",
            pack_key,
            &pack_hash,
        )
        .await?,
    )?;
    let pack = service::compile_context_pack_idempotent(
        &state,
        project.public_id,
        pack_input,
        &pack_lease,
    )
    .await?;
    assert_replay(
        claim(
            &state,
            Some(project_internal_id),
            "context_pack.compile",
            pack_key,
            &pack_hash,
        )
        .await?,
        &pack,
    )?;

    let handoff_input = CreateHandoff {
        source_session_id: product_session.session.public_id,
        context_pack_id: pack.public_id,
    };
    let handoff_hash = idempotency::hash_request(&handoff_input)?;
    let handoff_key = Uuid::new_v4();
    let handoff_lease = expect_new(
        claim(
            &state,
            Some(project_internal_id),
            "handoff.create",
            handoff_key,
            &handoff_hash,
        )
        .await?,
    )?;
    let handoff = service::create_handoff_idempotent(
        &state,
        project.public_id,
        handoff_input,
        &handoff_lease,
    )
    .await?;
    assert_replay(
        claim(
            &state,
            Some(project_internal_id),
            "handoff.create",
            handoff_key,
            &handoff_hash,
        )
        .await?,
        &handoff,
    )?;

    let plan_input = GenerateTechnicalPlan {
        session_id: handoff.target_session_public_id,
    };
    let plan_hash = idempotency::hash_request(&plan_input)?;
    let plan_key = Uuid::new_v4();
    let plan_lease = expect_new(
        claim(
            &state,
            Some(project_internal_id),
            "technical_plan.generate",
            plan_key,
            &plan_hash,
        )
        .await?,
    )?;
    let plan = service::generate_technical_plan_idempotent(
        &state,
        project.public_id,
        plan_input,
        &plan_lease,
    )
    .await?;
    assert_replay(
        claim(
            &state,
            Some(project_internal_id),
            "technical_plan.generate",
            plan_key,
            &plan_hash,
        )
        .await?,
        &plan,
    )?;

    let snapshot = service::snapshot(&state, project.public_id).await?;
    let knowledge = snapshot
        .knowledge
        .first()
        .context("expected confirmed knowledge")?;
    let revision_input = ReviseKnowledge {
        statement: format!("{} Révision atomique.", knowledge.statement),
        title: None,
        rationale: Some("Test de récupération après perte de réponse.".into()),
    };
    let revision_hash = idempotency::hash_request(&(knowledge.public_id, &revision_input))?;
    let revision_key = Uuid::new_v4();
    let revision_lease = expect_new(
        claim(
            &state,
            Some(project_internal_id),
            "knowledge.revise",
            revision_key,
            &revision_hash,
        )
        .await?,
    )?;
    let revision = service::revise_knowledge_for_project_idempotent(
        &state,
        project.public_id,
        knowledge.public_id,
        revision_input,
        &revision_lease,
    )
    .await?;
    assert_replay(
        claim(
            &state,
            Some(project_internal_id),
            "knowledge.revise",
            revision_key,
            &revision_hash,
        )
        .await?,
        &revision,
    )?;

    let mut counts_tx = begin_scoped_transaction(&state).await?;
    let (sessions, briefs, packs, handoffs, plans, revised_versions):
        (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        "select
           (select count(*) from app.sessions where project_id = $1),
           (select count(*) from app.deliverables where project_id = $1 and deliverable_type = 'feature-brief'),
           (select count(*) from app.context_packs where project_id = $1),
           (select count(*) from app.handoffs where project_id = $1),
           (select count(*) from app.deliverables where project_id = $1 and deliverable_type = 'technical-delivery-plan'),
           (select count(*) from app.knowledge_entry_versions v
              join app.knowledge_entries k on k.id = v.knowledge_entry_id
             where k.public_id = $2 and v.origin_type = 'human_revision')",
    )
    .bind(project_internal_id)
    .bind(knowledge.public_id)
    .fetch_one(&mut *counts_tx)
    .await?;
    counts_tx.commit().await?;
    ensure!(sessions == 2, "session replay created a duplicate");
    ensure!(briefs == 1, "brief replay created a duplicate");
    ensure!(packs == 1, "ContextPack replay created a duplicate");
    ensure!(handoffs == 1, "handoff replay created a duplicate");
    ensure!(plans == 1, "technical plan replay created a duplicate");
    ensure!(revised_versions == 1, "revision replay created a duplicate");

    Ok(())
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn provider_failure_classification_survives_durable_replay_and_resume() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@127.0.0.1:54322/postgres".into());
    let pool = PgPoolOptions::new()
        .max_connections(4)
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
        pool,
        engine: Arc::new(DeterministicEngine),
        workspace_id,
        workspace_internal_id: Some(workspace_internal_id),
        workspace_role,
        actor_id,
        agent_mode: "deterministic",
        steward_trigger: None,
    };

    let permanent_key = Uuid::new_v4();
    let permanent_hash = idempotency::hash_request(&json!({"prompt": "same-body"}))?;
    let permanent_lease = expect_new(
        claim(
            &state,
            None,
            "test.provider.permanent",
            permanent_key,
            &permanent_hash,
        )
        .await?,
    )?;
    let mut permanent_provider_calls = 0_u32;
    permanent_provider_calls += 1;
    let permanent_error = AppError::Provider(ProviderError::new(
        "OpenAI",
        ProviderErrorClass::Quota,
        1,
        Some(429),
    ));
    let mut permanent_tx = begin_scoped_transaction(&state).await?;
    let permanent_response = idempotency::fail(
        &mut permanent_tx,
        &permanent_lease,
        permanent_error.status_code().as_u16(),
        permanent_error.public_code(),
        if permanent_error.is_retryable_provider_failure() {
            FailureDisposition::Retryable
        } else {
            FailureDisposition::Permanent
        },
        json!({
            "code": permanent_error.public_code(),
            "message": permanent_error.public_message(),
        }),
    )
    .await?;
    permanent_tx.commit().await?;
    ensure!(!permanent_response.retryable);
    ensure!(
        permanent_response.status_code == 502,
        "quota failure must be stored as a permanent 502"
    );

    match claim(
        &state,
        None,
        "test.provider.permanent",
        permanent_key,
        &permanent_hash,
    )
    .await?
    {
        BeginOutcome::Replay { response, .. } => {
            ensure!(response == permanent_response, "permanent replay changed");
        }
        other => anyhow::bail!("permanent provider failure executed again: {other:?}"),
    }
    ensure!(
        permanent_provider_calls == 1,
        "permanent replay called the provider more than once"
    );
    let changed_hash = idempotency::hash_request(&json!({"prompt": "changed-body"}))?;
    ensure!(
        matches!(
            claim(
                &state,
                None,
                "test.provider.permanent",
                permanent_key,
                &changed_hash,
            )
            .await,
            Err(AppError::Conflict(_))
        ),
        "same permanent key with a changed body must conflict"
    );

    let transient_key = Uuid::new_v4();
    let transient_hash = idempotency::hash_request(&json!({"prompt": "retry-same-body"}))?;
    let transient_lease = expect_new(
        claim(
            &state,
            None,
            "test.provider.transient",
            transient_key,
            &transient_hash,
        )
        .await?,
    )?;
    let mut transient_provider_calls = 0_u32;
    transient_provider_calls += 1;
    let transient_error = AppError::Provider(ProviderError::new(
        "OpenAI",
        ProviderErrorClass::Server,
        3,
        Some(503),
    ));
    let mut transient_tx = begin_scoped_transaction(&state).await?;
    let transient_response = idempotency::fail(
        &mut transient_tx,
        &transient_lease,
        transient_error.status_code().as_u16(),
        transient_error.public_code(),
        if transient_error.is_retryable_provider_failure() {
            FailureDisposition::Retryable
        } else {
            FailureDisposition::Permanent
        },
        json!({
            "code": transient_error.public_code(),
            "message": transient_error.public_message(),
        }),
    )
    .await?;
    transient_tx.commit().await?;
    ensure!(transient_response.retryable);

    let retry_lease = match claim(
        &state,
        None,
        "test.provider.transient",
        transient_key,
        &transient_hash,
    )
    .await?
    {
        BeginOutcome::New {
            lease,
            reclaimed,
            retrying_transient_failure,
        } => {
            ensure!(reclaimed, "transient failure was not reclaimed");
            ensure!(
                retrying_transient_failure,
                "retry was not classified as a transient resume"
            );
            lease
        }
        other => anyhow::bail!("transient failure did not resume: {other:?}"),
    };
    transient_provider_calls += 1;
    let success_body = json!({"result": "provider-recovered"});
    let mut success_tx = begin_scoped_transaction(&state).await?;
    idempotency::complete(&mut success_tx, &retry_lease, 200, success_body.clone()).await?;
    success_tx.commit().await?;
    ensure!(
        transient_provider_calls == 2,
        "transient resume did not perform exactly one new provider call"
    );
    match claim(
        &state,
        None,
        "test.provider.transient",
        transient_key,
        &transient_hash,
    )
    .await?
    {
        BeginOutcome::Replay { response, .. } => {
            ensure!(!response.failed);
            ensure!(response.body == success_body);
        }
        other => anyhow::bail!("recovered provider result was not replayed: {other:?}"),
    }

    Ok(())
}

async fn claim(
    state: &AppState,
    project_id: Option<i64>,
    operation_key: &str,
    key: Uuid,
    request_hash: &str,
) -> Result<BeginOutcome, AppError> {
    let mut tx = begin_scoped_transaction(state).await?;
    let key = key.to_string();
    let outcome = idempotency::begin(
        &mut tx,
        BeginRequest {
            workspace_id: state
                .workspace_internal_id
                .expect("test state is request scoped"),
            project_id,
            actor_id: state.actor_id,
            operation_key,
            idempotency_key: &key,
            request_hash,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(outcome)
}

fn expect_new(outcome: BeginOutcome) -> Result<IdempotencyLease> {
    match outcome {
        BeginOutcome::New { lease, .. } => Ok(lease),
        other => anyhow::bail!("expected a new idempotency lease, got {other:?}"),
    }
}

fn assert_replay<T: Serialize>(outcome: BeginOutcome, expected: &T) -> Result<()> {
    let expected = serde_json::to_value(expected)?;
    match outcome {
        BeginOutcome::Replay { response, .. } => {
            ensure!(response.status_code == 200, "unexpected replay status");
            ensure!(!response.failed, "successful command replayed a failure");
            ensure!(response.body == expected, "replay body changed");
            Ok(())
        }
        other => anyhow::bail!("expected a durable replay, got {other:?}"),
    }
}

async fn begin_scoped_transaction(state: &AppState) -> Result<Transaction<'_, Postgres>, AppError> {
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "select set_config('app.current_actor_id', $1, true),
                set_config('app.current_workspace_id', $2, true),
                set_config('app.current_workspace_role', $3, true)",
    )
    .bind(state.actor_id.to_string())
    .bind(
        state
            .workspace_internal_id
            .expect("test state is request scoped")
            .to_string(),
    )
    .bind(&state.workspace_role)
    .execute(&mut *tx)
    .await?;
    Ok(tx)
}

async fn scoped_scalar<'a, T, P>(state: &AppState, query: &'a str, parameter: P) -> Result<T>
where
    T: for<'r> sqlx::Decode<'r, Postgres> + sqlx::Type<Postgres> + Send + Unpin,
    P: 'a + Send + sqlx::Encode<'a, Postgres> + sqlx::Type<Postgres>,
{
    let mut tx = begin_scoped_transaction(state).await?;
    let result = sqlx::query_scalar(query)
        .bind(parameter)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(result)
}
