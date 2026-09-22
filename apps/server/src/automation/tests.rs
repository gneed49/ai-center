use super::*;
use crate::{
    agent::DeterministicEngine,
    auth::RequestContext,
    company::{
        self,
        models::{CreateCompany, UpdateMember},
    },
};
use anyhow::{Context, Result, ensure};
use sqlx::postgres::PgPoolOptions;
use std::sync::atomic::{AtomicUsize, Ordering};

async fn isolated_actor() -> Result<AppState> {
    let raw = std::env::var("DATABASE_URL")
        .context("company_context requires the guarded isolated integration stack")?;
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
        "explicit safe integration mode required"
    );
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&raw)
        .await?;
    let (role, superuser, bypass): (String, bool, bool) = sqlx::query_as(
        "select current_user::text,rolsuper,rolbypassrls from pg_roles where rolname=current_user",
    )
    .fetch_one(&pool)
    .await?;
    ensure!(
        role == "ai_center_runtime" && !superuser && !bypass,
        "runtime RLS must be enforced"
    );
    Ok(AppState {
        pool,
        engine: Arc::new(DeterministicEngine),
        providers: None,
        workspace_id: Uuid::nil(),
        workspace_internal_id: None,
        workspace_role: "viewer".into(),
        actor_id: Uuid::new_v4(),
        agent_mode: "deterministic",
        steward_trigger: None,
    })
}

async fn select(state: &AppState, workspace: Uuid, actor: Uuid) -> Result<AppState> {
    let (internal, role): (i64, String) =
        sqlx::query_as("select workspace_id,role from app.authorize_workspace_member($1,$2)")
            .bind(workspace)
            .bind(actor)
            .fetch_one(&state.pool)
            .await?;
    Ok(state.scoped(&RequestContext {
        actor_id: actor,
        workspace_id: workspace,
        workspace_internal_id: Some(internal),
        workspace_role: role,
    }))
}

async fn create(state: &AppState, id: Uuid) -> Result<AppState> {
    let overview = company::create(
        state,
        CreateCompany {
            public_id: id,
            name: "[FICTIF] Société de test".into(),
            description: "Contexte partagé de test".into(),
        },
    )
    .await?;
    assert!(overview.setup_complete);
    assert_eq!(overview.agents.len(), 5);
    assert!(
        overview.projects.is_empty(),
        "company scope must not become a customer project"
    );
    select(state, id, state.actor_id).await
}

fn guarded(state: &AppState, hour: i64, concurrent: i64) -> Arc<ControlledEngine> {
    Arc::new(ControlledEngine {
        state: state.clone(),
        inner: Arc::new(DeterministicEngine),
        limits: Limits {
            calls_per_hour: hour,
            concurrent_calls: concurrent,
            per_actor_calls_per_hour: hour,
            per_actor_concurrent_calls: concurrent,
            call_timeout_seconds: 10,
        },
    })
}
struct Dropped(Arc<AtomicUsize>);
#[tokio::test]
#[ignore = "guarded PostgreSQL integration"]
async fn per_actor_quota_preserves_capacity_for_teammates_and_survives_reconnection() -> Result<()>
{
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let colleague = Uuid::new_v4();
    let mut tx = owner.begin_request().await?;
    sqlx::query("insert into app.workspace_members(workspace_id,actor_id,role,invitation_status,accepted_at) values(app.current_workspace_id(),$1,'editor','accepted',now())").bind(colleague).execute(&mut *tx).await?;
    tx.commit().await?;
    let editor = select(&owner, owner.workspace_id, colleague).await?;
    let limits = Limits {
        calls_per_hour: 10,
        concurrent_calls: 4,
        per_actor_calls_per_hour: 2,
        per_actor_concurrent_calls: 1,
        call_timeout_seconds: 10,
    };
    let owner_engine = ControlledEngine {
        state: owner.clone(),
        inner: Arc::new(DeterministicEngine),
        limits,
    };
    let editor_engine = ControlledEngine {
        state: editor.clone(),
        inner: Arc::new(DeterministicEngine),
        limits,
    };
    let (reserved, _) = owner_engine.reserve("respond").await?;
    let invoked = AtomicUsize::new(0);
    assert!(matches!(
        owner_engine
            .run("respond", async {
                invoked.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
            .await,
        Err(AppError::Capacity {
            retry_after_seconds: 5
        })
    ));
    assert_eq!(invoked.load(Ordering::SeqCst), 0);
    editor_engine.run("respond", async { Ok(()) }).await?;
    let mut tx = owner.begin_request().await?;
    sqlx::query("select app.finish_ai_call_reservation($1,'completed')")
        .bind(reserved)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    owner_engine.run("respond", async { Ok(()) }).await?;
    let reconnected = ControlledEngine {
        state: select(&owner, owner.workspace_id, owner.actor_id).await?,
        inner: Arc::new(DeterministicEngine),
        limits,
    };
    assert!(matches!(
        reconnected
            .run("respond", async {
                invoked.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
            .await,
        Err(AppError::Capacity {
            retry_after_seconds: 3600
        })
    ));
    assert_eq!(invoked.load(Ordering::SeqCst), 0);
    editor_engine.run("respond", async { Ok(()) }).await?;
    let usage = status(&owner).await?;
    assert_eq!(usage.calls_last_hour, 4);
    assert_eq!(usage.actor_calls_last_hour, 2);
    assert_eq!(usage.active_calls, 0);
    assert_eq!(usage.actor_active_calls, 0);
    Ok(())
}
impl Drop for Dropped {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
async fn wait_started(counter: &AtomicUsize, target: usize) -> Result<()> {
    tokio::time::timeout(Duration::from_secs(5), async {
        while counter.load(Ordering::SeqCst) < target {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await?;
    Ok(())
}
#[tokio::test]
#[ignore = "guarded PostgreSQL integration"]
async fn automation_limits_and_pause_are_durable_and_cancel_every_active_future() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let engine = guarded(&owner, 2, 2);
    let started = Arc::new(AtomicUsize::new(0));
    let dropped = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let engine = engine.clone();
        let started = started.clone();
        let dropped = dropped.clone();
        handles.push(tokio::spawn(async move {
            engine
                .run("respond", async move {
                    let _marker = Dropped(dropped);
                    started.fetch_add(1, Ordering::SeqCst);
                    std::future::pending::<AppResult<()>>().await
                })
                .await
        }));
    }
    wait_started(&started, 2).await?;
    let polled = AtomicUsize::new(0);
    let denied = guarded(&owner, 2, 2)
        .run("respond", async {
            polled.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .await;
    assert!(matches!(denied, Err(AppError::Capacity { .. })));
    assert_eq!(polled.load(Ordering::SeqCst), 0);
    let paused = set(
        &owner,
        SetControl {
            enabled: false,
            expected_generation: 0,
        },
        None,
    )
    .await?;
    assert_eq!(paused.generation, 1);
    for handle in handles {
        assert!(matches!(
            tokio::time::timeout(Duration::from_secs(4), handle).await??,
            Err(AppError::AutomationPaused)
        ));
    }
    assert_eq!(dropped.load(Ordering::SeqCst), 2);
    assert_eq!(status(&owner).await?.active_calls, 0);
    assert!(matches!(
        set(
            &owner,
            SetControl {
                enabled: true,
                expected_generation: 0
            },
            None
        )
        .await,
        Err(AppError::Conflict(_))
    ));
    set(
        &owner,
        SetControl {
            enabled: true,
            expected_generation: 1,
        },
        None,
    )
    .await?;
    assert!(
        matches!(
            guarded(&owner, 2, 2).run("respond", async { Ok(()) }).await,
            Err(AppError::Capacity { .. })
        ),
        "recreating a wrapper must not reset the quota"
    );
    let foreign = create(&actor, Uuid::new_v4()).await?;
    guarded(&foreign, 2, 2)
        .run("respond", async { Ok(()) })
        .await?;
    assert_eq!(status(&foreign).await?.calls_last_hour, 1);
    assert_eq!(status(&owner).await?.calls_last_hour, 2);
    owner.pool.close().await;
    Ok(())
}
#[tokio::test]
#[ignore = "guarded PostgreSQL integration"]
async fn automation_demotion_cancels_and_releases_without_viewer_control_rights() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let editor_actor = Uuid::new_v4();
    let mut tx = owner.begin_request().await?;
    let member:Uuid=sqlx::query_scalar("insert into app.workspace_members(workspace_id,actor_id,role,invitation_status,accepted_at) values(app.current_workspace_id(),$1,'editor','accepted',now()) returning public_id").bind(editor_actor).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    let editor = select(&owner, owner.workspace_id, editor_actor).await?;
    assert!(matches!(
        set(
            &editor,
            SetControl {
                enabled: false,
                expected_generation: 0
            },
            None
        )
        .await,
        Err(AppError::Forbidden)
    ));
    let engine = guarded(&editor, 4, 2);
    let started = Arc::new(AtomicUsize::new(0));
    let copy = started.clone();
    let handle = tokio::spawn(async move {
        engine
            .run("respond", async move {
                copy.fetch_add(1, Ordering::SeqCst);
                std::future::pending::<AppResult<()>>().await
            })
            .await
    });
    wait_started(&started, 1).await?;
    company::update_member(
        &owner,
        member,
        UpdateMember {
            role: Some("viewer".into()),
            invitation_status: None,
        },
        None,
    )
    .await?;
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(4), handle).await??,
        Err(AppError::Forbidden)
    ));
    let viewer = select(&owner, owner.workspace_id, editor_actor).await?;
    assert_eq!(status(&viewer).await?.active_calls, 0);
    assert!(matches!(
        guarded(&viewer, 4, 2)
            .run("respond", async { Ok(()) })
            .await,
        Err(AppError::Forbidden)
    ));
    owner.pool.close().await;
    Ok(())
}

#[tokio::test]
#[ignore = "guarded PostgreSQL integration"]
async fn private_bootstrap_authorization_requires_existing_ownership_or_operator_approval()
-> Result<()> {
    use crate::{auth::AuthRuntime, config::AuthMode};
    let actor = isolated_actor().await?;
    let auth = AuthRuntime::new(AuthMode::Supabase, Uuid::nil(), Uuid::nil(), None)?;
    assert!(
        !auth.may_create_company(actor.actor_id, &actor.pool).await?,
        "a verified identity alone must not allocate provider-funded companies"
    );
    let owner = create(&actor, Uuid::new_v4()).await?;
    assert!(auth.may_create_company(owner.actor_id, &owner.pool).await?);
    let viewer = Uuid::new_v4();
    let mut tx = owner.begin_request().await?;
    sqlx::query("insert into app.workspace_members(workspace_id,actor_id,role,invitation_status,accepted_at) values(app.current_workspace_id(),$1,'viewer','accepted',now())").bind(viewer).execute(&mut *tx).await?;
    tx.commit().await?;
    assert!(!auth.may_create_company(viewer, &owner.pool).await?);
    assert!(!auth.may_create_company(Uuid::nil(), &owner.pool).await?);
    owner.pool.close().await;
    Ok(())
}

#[tokio::test]
#[ignore = "guarded PostgreSQL integration"]
async fn automation_monitor_does_not_block_a_transport_using_one_database_connection() -> Result<()>
{
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let mut narrow = owner.clone();
    narrow.pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;
    let engine = guarded(&narrow, 10, 2);
    tokio::time::timeout(
        Duration::from_secs(5),
        engine.run("respond", async {
            let tx = narrow.begin_request().await?;
            tokio::time::sleep(Duration::from_millis(900)).await;
            tx.commit().await?;
            Ok(())
        }),
    )
    .await??;
    assert_eq!(status(&owner).await?.active_calls, 0);
    let started = Arc::new(AtomicUsize::new(0));
    let copy = started.clone();
    let scope = narrow.clone();
    let handle = tokio::spawn(async move {
        engine
            .run("respond", async move {
                let _held_connection = scope.begin_request().await?;
                copy.fetch_add(1, Ordering::SeqCst);
                std::future::pending::<AppResult<()>>().await
            })
            .await
    });
    wait_started(&started, 1).await?;
    set(
        &owner,
        SetControl {
            enabled: false,
            expected_generation: 0,
        },
        None,
    )
    .await?;
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(5), handle).await??,
        Err(AppError::AutomationInterrupted)
    ));
    assert_eq!(
        status(&owner).await?.active_calls,
        0,
        "the cancelled transport must release its connection before its reservation is settled"
    );
    narrow.pool.close().await;
    owner.pool.close().await;
    Ok(())
}

#[tokio::test]
#[ignore = "guarded PostgreSQL integration"]
async fn operation_deadline_is_a_failed_attempt_not_a_quota_deferral() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let engine = ControlledEngine {
        state: owner.clone(),
        inner: Arc::new(DeterministicEngine),
        limits: Limits {
            calls_per_hour: 10,
            concurrent_calls: 2,
            per_actor_calls_per_hour: 10,
            per_actor_concurrent_calls: 2,
            call_timeout_seconds: 1,
        },
    };
    let outcome = engine
        .run("respond", std::future::pending::<AppResult<()>>())
        .await;
    assert!(matches!(outcome, Err(AppError::AutomationInterrupted)));
    let mut tx = owner.begin_request().await?;
    let persisted: String = sqlx::query_scalar(
        "select status from app.ai_call_reservations where workspace_id=app.current_workspace_id()",
    )
    .fetch_one(&mut *tx)
    .await?;
    assert_eq!(persisted, "failed");
    tx.commit().await?;
    assert_eq!(status(&owner).await?.active_calls, 0);
    owner.pool.close().await;
    Ok(())
}
