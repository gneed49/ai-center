//! Explicit disposable-PostgreSQL contract. Run only with `DATABASE_URL` pointing
//! at the isolated integration stack; the GitHub origin is a loopback fixture.
use std::sync::{
    Arc,
    atomic::{AtomicU16, Ordering},
};
use std::time::Duration;

use super::*;
use crate::{
    agent::DeterministicEngine,
    integrations::github::GitHubClient,
    models::{
        CompileContextPack, CreateHandoff, CreateProject, CreateSession, DecideProposals,
        GenerateTechnicalPlan, ProposalDecision, SendMessage,
    },
    service,
};
use axum::{
    Json, Router,
    extract::{Request, State},
    http::StatusCode,
    response::IntoResponse,
};

mod observation_cycles;
mod tracking_tests;

const HEAD_SHA: &str = "2222222222222222222222222222222222222222";

async fn provider(
    State(mode): State<Arc<AtomicU16>>,
    request: Request,
) -> axum::response::Response {
    assert_eq!(
        request.method(),
        "GET",
        "observation never writes to GitHub"
    );
    let status = mode.load(Ordering::Relaxed);
    if status == 403 {
        return (
            StatusCode::FORBIDDEN,
            [("retry-after", "0")],
            Json(json!({"message":"secondary rate limit"})),
        )
            .into_response();
    }
    if status == 404 {
        return StatusCode::NOT_FOUND.into_response();
    }
    let head_sha = if status == 201 {
        "3333333333333333333333333333333333333333"
    } else {
        HEAD_SHA
    };
    let path = request.uri().path();
    let check_status = if status == 202 { "pending" } else { "success" };
    let value = if path.ends_with("/check-runs") {
        json!({"check_runs":[{"name":"ci","status":"completed","conclusion":"success"}]})
    } else if path.ends_with("/status") {
        json!({"sha":head_sha,"statuses":[{"context":"deploy","state":check_status,"target_url":"https://github.com/acme/context/actions/runs/1","updated_at":"2026-08-25T00:00:00Z"}]})
    } else if path.ends_with("/files") {
        json!([{"filename":"src/main.rs"}])
    } else if path.ends_with("/commits") {
        json!([{"sha":head_sha}])
    } else if path.contains("/pulls/") {
        json!({"title":"Observed delivery","state":"open","draft":false,"merged":false,"base":{"sha":"1111111111111111111111111111111111111111"},"head":{"sha":head_sha}})
    } else if path.contains("/commits/") {
        json!({"sha":head_sha,"files":[{"filename":"src/main.rs"}]})
    } else {
        json!({"full_name":"acme/context","archived":false})
    };
    ([("etag", "\"fixture-v1\"")], Json(value)).into_response()
}

#[tokio::test]
#[ignore = "requires the isolated PostgreSQL integration stack and loopback sockets"]
async fn github_persistence_preserves_proofs_under_rate_limits_and_binds_idempotence_to_target()
-> anyhow::Result<()> {
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await?;
    let actor_id = "00000000-0000-0000-0000-000000000001".parse()?;
    let workspace_id = "10000000-0000-0000-0000-000000000001".parse()?;
    let (workspace_internal_id, workspace_role): (i64, String) =
        sqlx::query_as("select workspace_id, role from app.authorize_workspace_member($1,$2)")
            .bind(workspace_id)
            .bind(actor_id)
            .fetch_one(&pool)
            .await?;
    if let Ok(expected_role) = std::env::var("AI_CENTER_EXPECT_DATABASE_ROLE") {
        let role: String = sqlx::query_scalar("select current_user")
            .fetch_one(&pool)
            .await?;
        assert_eq!(role, expected_role);
    }
    let state = AppState {
        pool,
        engine: Arc::new(DeterministicEngine),
        workspace_id,
        workspace_internal_id: Some(workspace_internal_id),
        workspace_role: workspace_role.clone(),
        actor_id,
        agent_mode: "deterministic",
        steward_trigger: None,
    };
    let context = RequestContext {
        actor_id,
        workspace_id,
        workspace_internal_id: Some(workspace_internal_id),
        workspace_role,
    };
    let project = service::create_project(
        &state,
        CreateProject {
            name: format!("GitHub contract {}", Uuid::new_v4()),
            objective:
                "Les références GitHub conservent leurs preuves pendant une limite transitoire."
                    .into(),
        },
    )
    .await?;
    let session = service::create_session(
        &state,
        project.public_id,
        CreateSession {
            node_key: "product".into(),
            title: Some("GitHub proof contract".into()),
        },
    )
    .await?;
    let turn = service::send_message(
        &state,
        session.session.public_id,
        SendMessage {
            content: "Les décisions confirmées restent disponibles, traçables et testables.".into(),
            client_message_id: Uuid::new_v4(),
        },
    )
    .await?;
    service::decide_proposals(
        &state,
        session.session.public_id,
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
    service::evaluate_product_gate(&state, project.public_id).await?;
    service::generate_feature_brief(&state, project.public_id).await?;
    let pack = service::compile_context_pack(
        &state,
        project.public_id,
        CompileContextPack {
            source_session_id: session.session.public_id,
            task_kind: "technical-delivery-plan".into(),
            token_budget: Some(12_000),
        },
    )
    .await?;
    let handoff = service::create_handoff(
        &state,
        project.public_id,
        CreateHandoff {
            source_session_id: session.session.public_id,
            context_pack_id: pack.public_id,
        },
    )
    .await?;
    let plan = service::generate_technical_plan(
        &state,
        project.public_id,
        GenerateTechnicalPlan {
            session_id: handoff.target_session_public_id,
        },
    )
    .await?;
    let mut tx = begin_scoped_transaction(&state, &context).await?;
    let admin_database_url = std::env::var("AI_CENTER_ADMIN_DATABASE_URL")?;
    let admin_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin_database_url)
        .await?;
    let connection: Uuid = sqlx::query_scalar("insert into app.tool_connections (workspace_id,provider,auth_mode,external_account_id,display_name,capabilities,secret_reference,configuration,status,created_by_actor_id) values ($1,'github','github_app',$2,'Loopback contract','{read}','test-fixture-only','{\"installation_id\":\"777\"}','active',$3) returning public_id")
        .bind(workspace_internal_id).bind(Uuid::new_v4().to_string()).bind(actor_id).fetch_one(&admin_pool).await?;
    let (requirement_id, section_id): (Uuid, Uuid) = sqlx::query_as("select requirement.public_id, section.public_id from app.requirement_coverage coverage join app.knowledge_entries requirement on requirement.id=coverage.requirement_entry_id join app.deliverable_sections section on section.id=coverage.deliverable_section_id join app.deliverables deliverable on deliverable.id=coverage.deliverable_id where deliverable.public_id=$1 limit 1")
        .bind(plan.public_id).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    let mode = Arc::new(AtomicU16::new(200));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let endpoint = format!("http://{}", listener.local_addr()?);
    let app = Router::new().fallback(provider).with_state(mode.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let github = GitHubRuntime {
        client: Arc::new(GitHubClient::for_contract_test(
            &endpoint,
            Duration::from_secs(2),
        )?),
        installation_id: 777,
    };
    let mut imported = vec![];
    for suffix in [
        String::new(),
        "/pull/7".into(),
        "/pull/8".into(),
        format!("/commit/{HEAD_SHA}"),
    ] {
        let input = CreateGitHubReference {
            url: format!("https://github.com/acme/context{suffix}"),
            tool_connection_id: Some(connection),
            tracking: None,
        };
        let key = Uuid::new_v4();
        let reference = create_github_reference(
            &state,
            &context,
            &github,
            project.public_id,
            key,
            input.clone(),
        )
        .await?;
        assert_eq!(
            reference,
            create_github_reference(&state, &context, &github, project.public_id, key, input)
                .await?
        );
        imported.push(reference);
    }
    assert_eq!(
        imported
            .iter()
            .map(|reference| reference.reference.object_kind.as_str())
            .collect::<Vec<_>>(),
        ["repository", "pull_request", "pull_request", "commit"]
    );
    let input = CreateExternalEvidence {
        artifact_id: None,
        requirement_id,
        deliverable_id: plan.public_id,
        deliverable_section_id: section_id,
        title: "Observed implementation".into(),
        description: "Human-reviewed fixture".into(),
    };
    assert!(
        create_evidence(
            &state,
            &context,
            imported[0].reference.public_id,
            Uuid::new_v4(),
            input.clone()
        )
        .await
        .is_err(),
        "mutable repository must not create revision-bound evidence"
    );
    let key = Uuid::new_v4();
    let reference_id = imported[1].reference.public_id;
    let evidence = create_evidence(&state, &context, reference_id, key, input.clone()).await?;
    assert_eq!(
        evidence,
        create_evidence(&state, &context, reference_id, key, input.clone()).await?
    );
    assert!(matches!(
        create_evidence(
            &state,
            &context,
            imported[2].reference.public_id,
            key,
            input.clone()
        )
        .await,
        Err(AppError::Conflict(_))
    ));
    assert!(
        get(&state, &context, imported[2].reference.public_id)
            .await?
            .evidences
            .is_empty()
    );
    let commit_evidence = create_evidence(
        &state,
        &context,
        imported[3].reference.public_id,
        Uuid::new_v4(),
        input.clone(),
    )
    .await?;
    assert_eq!(commit_evidence.evidence_type, "github_commit");
    let reviewed = review_evidence(
        &state,
        &context,
        reference_id,
        evidence.public_id,
        Uuid::new_v4(),
        ReviewExternalEvidence {
            decision: EvidenceDecision::Validate,
        },
    )
    .await?;
    assert_eq!(reviewed.status, "valid");
    let before = get(&state, &context, reference_id).await?;
    let coverage_before = coverage_status(&state, &context, evidence.public_id).await?;
    assert_eq!(coverage_before, "covered");
    mode.store(403, Ordering::Relaxed);
    let retry_key = Uuid::new_v4();
    assert!(matches!(
        refresh(&state, &context, &github, reference_id, retry_key).await,
        Err(AppError::ConnectorRateLimited { .. })
    ));
    assert_eq!(get(&state, &context, reference_id).await?, before);
    assert_eq!(
        coverage_status(&state, &context, evidence.public_id).await?,
        coverage_before
    );
    mode.store(200, Ordering::Relaxed);
    let recovered = refresh(&state, &context, &github, reference_id, retry_key).await?;
    assert_eq!(recovered.reference.sync_status, "current");
    assert_eq!(recovered.evidences[0].status, "valid");
    assert_eq!(
        coverage_status(&state, &context, evidence.public_id).await?,
        "covered"
    );
    mode.store(404, Ordering::Relaxed);
    let unavailable = refresh(&state, &context, &github, reference_id, Uuid::new_v4()).await?;
    assert_eq!(unavailable.reference.sync_status, "unavailable");
    assert_eq!(unavailable.evidences[0].status, "unavailable");
    assert_eq!(
        coverage_status(&state, &context, evidence.public_id).await?,
        "missing"
    );
    mode.store(200, Ordering::Relaxed);
    tracking_tests::exercise_tracking(
        &state,
        &context,
        &github,
        &mode,
        &admin_pool,
        project.public_id,
        pack.public_id,
        connection,
        input.clone(),
    )
    .await?;
    observation_cycles::exercise_cycles(
        &state,
        &context,
        &github,
        &mode,
        project.public_id,
        connection,
        input,
    )
    .await?;
    server.abort();
    Ok(())
}

async fn coverage_status(
    state: &AppState,
    context: &RequestContext,
    evidence_id: Uuid,
) -> AppResult<String> {
    let mut tx = begin_scoped_transaction(state, context).await?;
    let result = sqlx::query_scalar("select coverage.status from app.requirement_coverage coverage join app.evidences evidence on evidence.id=coverage.evidence_id where evidence.public_id=$1").bind(evidence_id).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(result)
}
