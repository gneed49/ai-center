use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use ai_center_server::{
    agent::{
        AgentEngine, AgentInput, ContextSelectionDraft, ContextSelectionInput, DeterministicEngine,
        EngineOutput, StewardInput, StewardOutput, TechnicalPlanDraft, TechnicalPlanInput,
    },
    error::{AppError, AppResult, ProviderError, ProviderErrorClass},
    models::{AgentTurn, CreateProject, CreateSession, SendMessage},
    service::{self, AppState},
};
use anyhow::{Result, ensure};
use async_trait::async_trait;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction, postgres::PgPoolOptions};
use uuid::Uuid;

struct ProbeEngine {
    pool: PgPool,
    actor_id: Uuid,
    workspace_internal_id: i64,
    workspace_role: String,
    session_public_id: Uuid,
    fail_respond: bool,
    running_seen: Arc<AtomicBool>,
}

#[derive(sqlx::FromRow)]
struct ModelRunRow {
    status: String,
    provider: String,
    model: String,
    prompt_version: String,
    schema_version: String,
    source_public_ids: Vec<Uuid>,
    estimated_cost: Option<f64>,
    attempt_count: i32,
    lifecycle_complete: bool,
    error_class: Option<String>,
    error_message: Option<String>,
    output: Option<Value>,
}

impl ProbeEngine {
    async fn observe_running_run(&self) -> AppResult<()> {
        let mut tx = begin_scoped_transaction(
            &self.pool,
            self.actor_id,
            self.workspace_internal_id,
            &self.workspace_role,
        )
        .await?;
        let running: bool = sqlx::query_scalar(
            "select exists(
               select 1 from app.model_runs run
               join app.sessions session on session.id = run.session_id
               where session.public_id = $1
                 and run.operation = 'extract_knowledge'
                 and run.status = 'running'
             )",
        )
        .bind(self.session_public_id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        self.running_seen.store(running, Ordering::SeqCst);
        Ok(())
    }
}

#[async_trait]
impl AgentEngine for ProbeEngine {
    fn provider_name(&self) -> &'static str {
        if self.fail_respond {
            "openai"
        } else {
            "deterministic"
        }
    }

    fn requested_model(&self) -> &'static str {
        if self.fail_respond {
            "probe-model"
        } else {
            "deterministic-test-double"
        }
    }

    async fn respond(&self, input: AgentInput) -> AppResult<EngineOutput<AgentTurn>> {
        self.observe_running_run().await?;
        if self.fail_respond {
            Err(ProviderError::new("OpenAI", ProviderErrorClass::Server, 3, Some(503)).into())
        } else {
            DeterministicEngine.respond(input).await
        }
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

    async fn analyze_contradictions(
        &self,
        input: StewardInput,
    ) -> AppResult<EngineOutput<StewardOutput>> {
        DeterministicEngine.analyze_contradictions(input).await
    }
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn provider_failure_and_success_leave_terminal_model_runs() -> Result<()> {
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
    let bootstrap = AppState {
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
        &bootstrap,
        CreateProject {
            name: format!("Model run lifecycle {}", Uuid::new_v4()),
            objective: "Prouver le cycle de vie durable des appels fournisseur.".into(),
        },
    )
    .await?;
    let session = service::create_session(
        &bootstrap,
        project.public_id,
        CreateSession {
            node_key: "product".into(),
            title: Some("Model run probe".into()),
        },
    )
    .await?;

    let failed_running_seen = Arc::new(AtomicBool::new(false));
    let failing_state = AppState {
        engine: Arc::new(ProbeEngine {
            pool: pool.clone(),
            actor_id,
            workspace_internal_id,
            workspace_role: workspace_role.clone(),
            session_public_id: session.session.public_id,
            fail_respond: true,
            running_seen: Arc::clone(&failed_running_seen),
        }),
        agent_mode: "openai",
        ..bootstrap.clone()
    };
    let failure = service::send_message(
        &failing_state,
        session.session.public_id,
        SendMessage {
            content: "Déclencher le double fournisseur en échec.".into(),
            client_message_id: Uuid::new_v4(),
        },
    )
    .await
    .expect_err("the provider double must fail");
    assert!(matches!(failure, AppError::Provider(_)));
    assert!(
        failed_running_seen.load(Ordering::SeqCst),
        "the running model_run must be committed before the provider is called"
    );

    let completed_running_seen = Arc::new(AtomicBool::new(false));
    let succeeding_state = AppState {
        engine: Arc::new(ProbeEngine {
            pool: pool.clone(),
            actor_id,
            workspace_internal_id,
            workspace_role: workspace_role.clone(),
            session_public_id: session.session.public_id,
            fail_respond: false,
            running_seen: Arc::clone(&completed_running_seen),
        }),
        agent_mode: "deterministic",
        ..bootstrap.clone()
    };
    service::send_message(
        &succeeding_state,
        session.session.public_id,
        SendMessage {
            content: "Confirmer le chemin fournisseur réussi.".into(),
            client_message_id: Uuid::new_v4(),
        },
    )
    .await?;
    assert!(
        completed_running_seen.load(Ordering::SeqCst),
        "the successful provider call must also observe its committed running run"
    );

    let mut verification =
        begin_scoped_transaction(&pool, actor_id, workspace_internal_id, &workspace_role).await?;
    let rows = sqlx::query_as::<_, ModelRunRow>(
        "select run.status, run.provider, run.model, run.prompt_version,
                run.schema_version, run.source_public_ids,
                run.estimated_cost::double precision, run.attempt_count,
                run.started_at is not null and run.completed_at is not null
                  as lifecycle_complete,
                run.error_class, run.error_message, run.output
         from app.model_runs run
         join app.sessions session on session.id = run.session_id
         where session.public_id = $1 and run.operation = 'extract_knowledge'
         order by run.id",
    )
    .bind(session.session.public_id)
    .fetch_all(&mut *verification)
    .await?;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].status, "failed");
    assert_eq!(rows[0].provider, "openai");
    assert_eq!(rows[0].model, "probe-model");
    assert_eq!(rows[0].prompt_version, "alpha-agent-turn-v1");
    assert_eq!(rows[0].schema_version, "alpha-agent-turn-v1");
    assert!(rows[0].source_public_ids.is_empty());
    assert_eq!(rows[0].estimated_cost, None);
    assert_eq!(rows[0].attempt_count, 3);
    assert!(rows[0].lifecycle_complete);
    assert_eq!(rows[0].error_class.as_deref(), Some("provider_server"));
    assert_eq!(
        rows[0].error_message.as_deref(),
        Some("OpenAI provider server failure after 3 attempt(s)")
    );
    assert!(rows[0].output.is_none());

    assert_eq!(rows[1].status, "completed");
    assert_eq!(rows[1].provider, "deterministic");
    assert_eq!(rows[1].model, "deterministic-test-double");
    assert_eq!(rows[1].prompt_version, "alpha-agent-turn-v1");
    assert_eq!(rows[1].schema_version, "alpha-agent-turn-v1");
    assert!(rows[1].source_public_ids.is_empty());
    assert_eq!(rows[1].estimated_cost, Some(0.0));
    assert_eq!(rows[1].attempt_count, 1);
    assert!(rows[1].lifecycle_complete);
    assert!(rows[1].error_class.is_none());
    assert!(rows[1].error_message.is_none());
    assert!(rows[1].output.is_some());

    sqlx::query(
        "update app.projects set status = 'archived'
         where public_id = $1 and workspace_id = $2",
    )
    .bind(project.public_id)
    .bind(workspace_internal_id)
    .execute(&mut *verification)
    .await?;
    verification.commit().await?;
    Ok(())
}

async fn begin_scoped_transaction<'a>(
    pool: &'a PgPool,
    actor_id: Uuid,
    workspace_id: i64,
    workspace_role: &str,
) -> AppResult<Transaction<'a, Postgres>> {
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
