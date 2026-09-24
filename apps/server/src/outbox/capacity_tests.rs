//! Guarded database regressions; loaded when the capacity schema is integrated.
use super::*;
use crate::{
    agent::DeterministicEngine,
    auth::RequestContext,
    company::{self, models::CreateCompany},
    service::AppState,
};
use anyhow::{Context, Result, ensure};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

async fn isolated_company() -> Result<AppState> {
    let raw = std::env::var("DATABASE_URL").context("guarded integration stack required")?;
    let url = url::Url::parse(&raw).map_err(|_| anyhow::anyhow!("invalid integration URL"))?;
    ensure!(
        url.scheme() == "postgresql"
            && url.host_str() == Some("127.0.0.1")
            && url.port() == Some(55322)
            && url.username() == "ai_center_runtime"
            && url.path() == "/postgres"
            && url.query().is_none()
            && url.fragment().is_none(),
        "isolated runtime database on 55322 required"
    );
    ensure!(
        std::env::var("AI_CENTER_EXPECT_DATABASE_ROLE").as_deref() == Ok("ai_center_runtime")
            && std::env::var("AI_CENTER_AGENT_MODE").as_deref() == Ok("deterministic"),
        "explicit isolated deterministic mode required"
    );
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&raw)
        .await?;
    let posture:(String,bool)=sqlx::query_as("select current_user::text,rolsuper or rolbypassrls from pg_roles where rolname=current_user")
        .fetch_one(&pool).await?;
    ensure!(
        posture == ("ai_center_runtime".into(), false),
        "runtime RLS required"
    );
    let state = AppState {
        pool,
        engine: Arc::new(DeterministicEngine),
        providers: None,
        workspace_id: Uuid::nil(),
        workspace_internal_id: None,
        workspace_role: "viewer".into(),
        actor_id: Uuid::new_v4(),
        agent_mode: "deterministic",
        steward_trigger: None,
    };
    let workspace = Uuid::new_v4();
    company::create(
        &state,
        CreateCompany {
            public_id: workspace,
            name: "[FICTIF] Reports de capacité".into(),
            description: String::new(),
        },
    )
    .await?;
    let (workspace_internal_id, workspace_role): (i64, String) =
        sqlx::query_as("select workspace_id,role from app.authorize_workspace_member($1,$2)")
            .bind(workspace)
            .bind(state.actor_id)
            .fetch_one(&state.pool)
            .await?;
    Ok(state.scoped(&RequestContext {
        actor_id: state.actor_id,
        workspace_id: workspace,
        workspace_internal_id: Some(workspace_internal_id),
        workspace_role,
    }))
}

async fn claim(state: &AppState, outbox: &Outbox) -> Result<ClaimedDomainEvent> {
    let mut tx = state.begin_request().await?;
    let event = outbox
        .claim_batch(
            &mut tx,
            state.workspace_internal_id.context("workspace")?,
            &["test.capacity".into()],
            "same-worker",
            1,
        )
        .await?
        .pop()
        .context("due event")?;
    tx.commit().await?;
    Ok(event)
}
async fn make_due(state: &AppState, id: Uuid) -> Result<()> {
    let mut tx = state.begin_request().await?;
    sqlx::query("update app.domain_events set available_at=clock_timestamp()-interval '1 second' where public_id=$1")
        .bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
async fn deferred_event(state: &AppState, outbox: &Outbox) -> Result<Uuid> {
    let mut tx = state.begin_request().await?;
    let id:Uuid=sqlx::query_scalar("insert into app.domain_events(workspace_id,event_type,aggregate_kind,aggregate_public_id) values(app.current_workspace_id(),'test.capacity','workspace',$1) returning public_id")
        .bind(state.workspace_id).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    for attempt in 1..=6 {
        let event = claim(state, outbox).await?;
        assert_eq!(event.lease_attempt, attempt);
        let mut tx = state.begin_request().await?;
        outbox
            .defer_for_admission(
                &mut tx,
                id,
                "same-worker",
                event.lease_attempt,
                AdmissionDeferral::Capacity(Duration::from_secs(3600)),
            )
            .await?;
        let row:(String,i32,i32,bool)=sqlx::query_as("select status,attempt_count,deferred_count,available_at>=clock_timestamp()+interval '3599 seconds' from app.domain_events where public_id=$1")
            .bind(id).fetch_one(&mut *tx).await?;
        assert_eq!(row, ("pending".into(), attempt, attempt, true));
        assert!(
            outbox
                .claim_batch(
                    &mut tx,
                    event.workspace_id,
                    &["test.capacity".into()],
                    "same-worker",
                    1
                )
                .await?
                .is_empty()
        );
        assert!(matches!(
            outbox
                .mark_processed(&mut tx, id, "same-worker", event.lease_attempt)
                .await,
            Err(OutboxError::LeaseLost { .. })
        ));
        tx.commit().await?;
        make_due(state, id).await?;
    }
    Ok(id)
}

#[tokio::test]
#[ignore = "guarded PostgreSQL integration"]
async fn quota_reports_preserve_the_failure_budget_and_monotonic_fencing() -> Result<()> {
    let state = isolated_company().await?;
    let outbox = Outbox::new(OutboxPolicy {
        max_attempts: 2,
        ..OutboxPolicy::default()
    })?;
    let id = deferred_event(&state, &outbox).await?;
    let event = claim(&state, &outbox).await?;
    assert_eq!(event.lease_attempt, 7);
    let mut tx = state.begin_request().await?;
    assert!(matches!(
        outbox.mark_processed(&mut tx, id, "same-worker", 6).await,
        Err(OutboxError::LeaseLost { .. })
    ));
    assert!(matches!(
        outbox.renew_lease(&mut tx, id, "same-worker", 6).await,
        Err(OutboxError::LeaseLost { .. })
    ));
    assert_eq!(
        outbox
            .mark_failed(
                &mut tx,
                id,
                "same-worker",
                7,
                "provider_failed",
                "Synthetic failure"
            )
            .await?,
        FailureDisposition::RetryScheduled {
            delay: Duration::from_secs(30)
        }
    );
    tx.commit().await?;
    make_due(&state, id).await?;
    let event = claim(&state, &outbox).await?;
    let mut tx = state.begin_request().await?;
    assert_eq!(
        outbox
            .mark_failed(
                &mut tx,
                id,
                "same-worker",
                event.lease_attempt,
                "provider_failed",
                "Synthetic failure"
            )
            .await?,
        FailureDisposition::DeadLettered
    );
    tx.commit().await?;
    state.pool.close().await;
    Ok(())
}

#[tokio::test]
#[ignore = "guarded PostgreSQL integration"]
async fn crash_reclaim_counts_real_attempts_after_capacity_reports() -> Result<()> {
    let state = isolated_company().await?;
    let outbox = Outbox::new(OutboxPolicy {
        max_attempts: 2,
        ..OutboxPolicy::default()
    })?;
    let id = deferred_event(&state, &outbox).await?;
    let event = claim(&state, &outbox).await?;
    let mut tx = state.begin_request().await?;
    sqlx::query("update app.domain_events set locked_until=clock_timestamp()-interval '1 second' where public_id=$1")
        .bind(id).execute(&mut *tx).await?;
    let reclaimed = outbox
        .reclaim_expired_leases(&mut tx, event.workspace_id, &["test.capacity".into()], 1)
        .await?;
    assert_eq!(
        reclaimed,
        ReclaimSummary {
            retry_scheduled: 1,
            dead_lettered: 0
        }
    );
    tx.commit().await?;
    state.pool.close().await;
    Ok(())
}
