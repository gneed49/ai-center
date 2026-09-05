use super::*;
use ai_center_server::idempotency::{self, BeginOutcome, BeginRequest, IdempotencyLease};
use std::time::Duration;

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn active_provider_renews_lease_and_lost_ownership_cancels_with_terminal_run() -> Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(6)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;
    let actor_id: Uuid = "00000000-0000-0000-0000-000000000001".parse()?;
    let workspace_id: Uuid = "10000000-0000-0000-0000-000000000001".parse()?;
    let (workspace_internal_id, workspace_role): (i64, String) =
        sqlx::query_as("select workspace_id, role from app.authorize_workspace_member($1,$2)")
            .bind(workspace_id)
            .bind(actor_id)
            .fetch_one(&pool)
            .await?;
    let mut state = AppState {
        pool: pool.clone(),
        engine: Arc::new(DeterministicEngine),
        actor_id,
        workspace_id,
        workspace_internal_id: Some(workspace_internal_id),
        workspace_role: workspace_role.clone(),
        agent_mode: "deterministic",
        steward_trigger: None,
    };
    let project = service::create_project(
        &state,
        CreateProject {
            name: format!("Lease renewal {}", Uuid::new_v4()),
            objective: "Une commande longue reste unique.".into(),
        },
    )
    .await?;
    let session = service::create_session(
        &state,
        project.public_id,
        CreateSession {
            node_key: "product".into(),
            title: None,
        },
    )
    .await?;
    let pause = Arc::new(ProviderPause {
        started: tokio::sync::Notify::new(),
        release: tokio::sync::Notify::new(),
        calls: std::sync::atomic::AtomicUsize::new(0),
    });
    state.engine = Arc::new(ProbeEngine {
        pool: pool.clone(),
        actor_id,
        workspace_internal_id,
        workspace_role: workspace_role.clone(),
        session_public_id: session.session.public_id,
        fail_respond: false,
        running_seen: Arc::new(AtomicBool::new(false)),
        pause: Some(pause.clone()),
    });
    let mut tx = scoped(&state).await?;
    let project_id: i64 = sqlx::query_scalar("select id from app.projects where public_id=$1")
        .bind(project.public_id)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    let input = SendMessage {
        content: "Les décisions doivent rester traçables et testables.".into(),
        client_message_id: Uuid::new_v4(),
    };
    let key = Uuid::new_v4();
    let hash = idempotency::hash_request(&input)?;
    let lease = new_lease(claim(&state, project_id, key, &hash).await?)?;
    let worker_state = state.clone();
    let worker_lease = lease.clone();
    let project_public_id = project.public_id;
    let session_id = session.session.public_id;
    let worker = tokio::spawn(async move {
        service::send_message_for_project_idempotent(
            &worker_state,
            project_public_id,
            session_id,
            input,
            &worker_lease,
        )
        .await
    });
    pause.started.notified().await;
    tokio::time::sleep(Duration::from_secs(31)).await;
    let concurrent = claim(&state, project_id, key, &hash).await?;
    assert!(
        matches!(concurrent, BeginOutcome::InProgress { locked_until, .. } if locked_until > lease.locked_until)
    );
    assert_eq!(pause.calls.load(Ordering::SeqCst), 1);
    pause.release.notify_one();
    worker.await??;
    assert!(matches!(
        claim(&state, project_id, key, &hash).await?,
        BeginOutcome::Replay { .. }
    ));
    let mut tx = scoped(&state).await?;
    let completed: i64 = sqlx::query_scalar(
        "select count(*) from app.model_runs where project_id=$1 and status='completed'",
    )
    .bind(project_id)
    .fetch_one(&mut *tx)
    .await?;
    assert_eq!(completed, 1);
    tx.commit().await?;

    let second_input = SendMessage {
        content: "Autre décision testable.".into(),
        client_message_id: Uuid::new_v4(),
    };
    let second_key = Uuid::new_v4();
    let second_hash = idempotency::hash_request(&second_input)?;
    let second_lease = new_lease(claim(&state, project_id, second_key, &second_hash).await?)?;
    let worker_state = state.clone();
    let worker_lease = second_lease.clone();
    let worker = tokio::spawn(async move {
        service::send_message_for_project_idempotent(
            &worker_state,
            project_public_id,
            session_id,
            second_input,
            &worker_lease,
        )
        .await
    });
    pause.started.notified().await;
    let mut tx = scoped(&state).await?;
    sqlx::query("update app.idempotency_records set locked_until=now()-interval '1 second' where public_id=$1")
        .bind(second_lease.record_public_id).execute(&mut *tx).await?;
    tx.commit().await?;
    let reclaimed = new_lease(claim(&state, project_id, second_key, &second_hash).await?)?;
    assert_ne!(reclaimed.generation, second_lease.generation);
    let stopped = tokio::time::timeout(Duration::from_secs(12), worker).await??;
    assert!(matches!(stopped, Err(AppError::Conflict(_))));
    let mut tx = scoped(&state).await?;
    let statuses: Vec<String> =
        sqlx::query_scalar("select status from app.model_runs where project_id=$1 order by id")
            .bind(project_id)
            .fetch_all(&mut *tx)
            .await?;
    assert_eq!(statuses, ["completed", "failed"]);
    assert!(matches!(
        idempotency::renew(&mut tx, &second_lease).await,
        Err(AppError::Conflict(_))
    ));
    tx.rollback().await?;
    assert_eq!(pause.calls.load(Ordering::SeqCst), 2);
    Ok(())
}

async fn scoped(state: &AppState) -> AppResult<Transaction<'_, Postgres>> {
    begin_scoped_transaction(
        &state.pool,
        state.actor_id,
        state.workspace_internal_id.unwrap(),
        &state.workspace_role,
    )
    .await
}

async fn claim(
    state: &AppState,
    project_id: i64,
    key: Uuid,
    hash: &str,
) -> AppResult<BeginOutcome> {
    let mut tx = scoped(state).await?;
    let result = idempotency::begin(
        &mut tx,
        BeginRequest {
            workspace_id: state.workspace_internal_id.unwrap(),
            project_id: Some(project_id),
            actor_id: state.actor_id,
            operation_key: "session.message.send",
            idempotency_key: &key.to_string(),
            request_hash: hash,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}

fn new_lease(outcome: BeginOutcome) -> Result<IdempotencyLease> {
    if let BeginOutcome::New { lease, .. } = outcome {
        Ok(lease)
    } else {
        anyhow::bail!("expected new lease")
    }
}
