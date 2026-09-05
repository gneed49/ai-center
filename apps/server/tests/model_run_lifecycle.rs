use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use ai_center_server::{
    agent::{
        AgentEngine, AgentInput, ContextSelectionDraft, ContextSelectionInput,
        CoverageEvaluationDraft, CoverageEvaluationInput, DeterministicEngine, EngineOutput,
        StewardInput, StewardOutput, TechnicalPlanDraft, TechnicalPlanInput,
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

#[path = "model_run_lifecycle/lease_tests.rs"]
mod lease_tests;

struct ProviderPause {
    started: tokio::sync::Notify,
    release: tokio::sync::Notify,
    calls: std::sync::atomic::AtomicUsize,
}

struct ProbeEngine {
    pool: PgPool,
    actor_id: Uuid,
    workspace_internal_id: i64,
    workspace_role: String,
    session_public_id: Uuid,
    fail_respond: bool,
    running_seen: Arc<AtomicBool>,
    pause: Option<Arc<ProviderPause>>,
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
        if let Some(pause) = &self.pause {
            pause.calls.fetch_add(1, Ordering::SeqCst);
            pause.started.notify_one();
            pause.release.notified().await;
        }
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
            pause: None,
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
            pause: None,
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

#[derive(Clone, Copy)]
enum CoverageMode {
    Success,
    ProviderFailure,
    MissingRequirement,
    DuplicateRequirement,
    ForeignSource,
    ConcurrentRevision,
}

struct CoverageProbeEngine {
    base: AppState,
    project_public_id: Uuid,
    session_public_id: Uuid,
    expected_pack: Value,
    requirement_public_id: Uuid,
    mode: CoverageMode,
    running_seen: Arc<AtomicBool>,
}

#[async_trait]
impl AgentEngine for CoverageProbeEngine {
    fn provider_name(&self) -> &'static str {
        "deterministic"
    }
    fn requested_model(&self) -> &'static str {
        "deterministic-test-double"
    }
    async fn respond(&self, input: AgentInput) -> AppResult<EngineOutput<AgentTurn>> {
        DeterministicEngine.respond(input).await
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
        let mut result = DeterministicEngine.generate_technical_plan(input).await?;
        // A model opinion is never allowed to become effective evidence.
        for item in &mut result.output.coverage {
            item.status = "covered".into();
        }
        Ok(result)
    }
    async fn analyze_contradictions(
        &self,
        input: StewardInput,
    ) -> AppResult<EngineOutput<StewardOutput>> {
        DeterministicEngine.analyze_contradictions(input).await
    }
    async fn evaluate_coverage(
        &self,
        input: CoverageEvaluationInput,
    ) -> AppResult<EngineOutput<CoverageEvaluationDraft>> {
        if input.context_pack != self.expected_pack {
            return Err(AppError::Internal(
                "coverage input diverged from its explicit pack".into(),
            ));
        }
        // The business pool has one connection. Acquiring it here proves that
        // the provider call is not holding a transaction/connection open.
        let mut tx = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            begin_scoped_transaction(
                &self.base.pool,
                self.base.actor_id,
                self.base.workspace_internal_id.expect("scoped fixture"),
                &self.base.workspace_role,
            ),
        )
        .await
        .map_err(|_| {
            AppError::Internal("coverage call held a database transaction open".into())
        })??;
        let observed: bool = sqlx::query_scalar(
            "select exists(select 1 from app.model_runs r join app.sessions s on s.id=r.session_id
               where s.public_id=$1 and r.operation='assess_coverage' and r.status='running')
             and exists(select 1 from app.model_runs r join app.sessions s on s.id=r.session_id
               where s.public_id=$1 and r.operation='generate_technical_plan' and r.status='completed')",
        ).bind(self.session_public_id).fetch_one(&mut *tx).await?;
        self.running_seen.store(observed, Ordering::SeqCst);
        tx.commit().await?;
        if matches!(self.mode, CoverageMode::ProviderFailure) {
            return Err(
                ProviderError::new("OpenAI", ProviderErrorClass::Server, 2, Some(503)).into(),
            );
        }
        let mut result = DeterministicEngine.evaluate_coverage(input).await?;
        result.metadata.input_tokens = Some(17);
        result.metadata.output_tokens = Some(9);
        match self.mode {
            CoverageMode::MissingRequirement => {
                result.output.requirements.clear();
            }
            CoverageMode::DuplicateRequirement => {
                result
                    .output
                    .requirements
                    .push(result.output.requirements[0].clone());
            }
            CoverageMode::ForeignSource => {
                result.output.requirements[0]
                    .source_version_ids
                    .push(Uuid::new_v4());
            }
            CoverageMode::ConcurrentRevision => {
                service::revise_knowledge_for_project(
                    &self.base,
                    self.project_public_id,
                    self.requirement_public_id,
                    ai_center_server::models::ReviseKnowledge {
                        statement: "Nouvelle exigence confirmée pendant l’évaluation".into(),
                        title: None,
                        rationale: None,
                    },
                )
                .await?;
            }
            CoverageMode::Success | CoverageMode::ProviderFailure => {}
        }
        Ok(result)
    }
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn independent_coverage_run_is_atomic_scoped_and_never_validates_evidence() -> Result<()> {
    use ai_center_server::models::{
        CompileContextPack, CreateHandoff, DecideProposals, GenerateTechnicalPlan, ProposalDecision,
    };
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;
    let workspace_id = "10000000-0000-0000-0000-000000000001".parse()?;
    let actor_id = "00000000-0000-0000-0000-000000000001".parse()?;
    let (workspace_internal_id, workspace_role): (i64, String) =
        sqlx::query_as("select workspace_id, role from app.authorize_workspace_member($1,$2)")
            .bind(workspace_id)
            .bind(actor_id)
            .fetch_one(&pool)
            .await?;
    let base = AppState {
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
        &base,
        CreateProject {
            name: format!("Coverage lifecycle {}", Uuid::new_v4()),
            objective: "Comparer un plan à ses exigences sans inventer de preuve".into(),
        },
    )
    .await?;
    let session = service::create_session(
        &base,
        project.public_id,
        CreateSession {
            node_key: "product".into(),
            title: None,
        },
    )
    .await?;
    let turn = service::send_message(
        &base,
        session.session.public_id,
        SendMessage {
            content: "Chaque réservation confirmée reste consultable.".into(),
            client_message_id: Uuid::new_v4(),
        },
    )
    .await?;
    service::decide_proposals(
        &base,
        session.session.public_id,
        DecideProposals {
            proposal_ids: turn.proposals.iter().map(|item| item.public_id).collect(),
            decision: ProposalDecision::Confirm,
        },
    )
    .await?;
    let pack = service::compile_context_pack(
        &base,
        project.public_id,
        CompileContextPack {
            source_session_id: session.session.public_id,
            task_kind: "technical-delivery-plan".into(),
            token_budget: None,
        },
    )
    .await?;
    let requirement_public_id: Uuid = pack.content["knowledge"]
        .as_array()
        .expect("pack knowledge")
        .iter()
        .find(|item| item["entry_type"] == "requirement")
        .expect("requirement")["knowledge_public_id"]
        .as_str()
        .expect("knowledge UUID")
        .parse()?;
    let handoff = service::create_handoff(
        &base,
        project.public_id,
        CreateHandoff {
            source_session_id: session.session.public_id,
            context_pack_id: pack.public_id,
        },
    )
    .await?;
    for mode in [
        CoverageMode::Success,
        CoverageMode::ProviderFailure,
        CoverageMode::MissingRequirement,
        CoverageMode::DuplicateRequirement,
        CoverageMode::ForeignSource,
        CoverageMode::ConcurrentRevision,
    ] {
        let running_seen = Arc::new(AtomicBool::new(false));
        let state = AppState {
            engine: Arc::new(CoverageProbeEngine {
                base: base.clone(),
                project_public_id: project.public_id,
                session_public_id: handoff.target_session_public_id,
                expected_pack: pack.content.clone(),
                requirement_public_id,
                mode,
                running_seen: Arc::clone(&running_seen),
            }),
            ..base.clone()
        };
        let result = service::generate_technical_plan(
            &state,
            project.public_id,
            GenerateTechnicalPlan {
                session_id: handoff.target_session_public_id,
            },
        )
        .await;
        ensure!(
            running_seen.load(Ordering::SeqCst),
            "independent coverage run was not durable before provider invocation"
        );
        if matches!(mode, CoverageMode::Success) {
            let plan = result?;
            ensure!(plan.coverage_status == "missing");
            ensure!(plan.content["coverage"][0]["status"] == "missing");
            ensure!(
                plan.content["coverage_assessment"]["requirements"][0]["assessment"] == "planned"
            );
        } else if matches!(mode, CoverageMode::ConcurrentRevision) {
            ensure!(matches!(result, Err(AppError::Conflict(_))));
        } else {
            ensure!(
                result.is_err(),
                "invalid coverage must abort the deliverable command"
            );
        }
        let mut tx =
            begin_scoped_transaction(&pool, actor_id, workspace_internal_id, &workspace_role)
                .await?;
        let (status, prompt, schema, input_tokens, output_tokens, sources): (String, String, String, Option<i32>, Option<i32>, Vec<Uuid>) = sqlx::query_as(
            "select r.status,r.prompt_version,r.schema_version,r.input_tokens,r.output_tokens,r.source_public_ids
             from app.model_runs r join app.sessions s on s.id=r.session_id
             where s.public_id=$1 and r.operation='assess_coverage' order by r.id desc limit 1",
        ).bind(handoff.target_session_public_id).fetch_one(&mut *tx).await?;
        ensure!(
            status
                == if matches!(mode, CoverageMode::Success) {
                    "completed"
                } else {
                    "failed"
                }
        );
        ensure!(prompt == "alpha-coverage-v1" && schema == "alpha-coverage-v1");
        ensure!(!sources.is_empty());
        if !matches!(mode, CoverageMode::ProviderFailure) {
            ensure!(input_tokens == Some(17) && output_tokens == Some(9));
        }
        let (deliverables, evidence, covered, linked_runs): (i64, i64, i64, i64) = sqlx::query_as(
            "select
               (select count(*) from app.deliverables d where d.project_id=p.id),
               (select count(*) from app.evidences e where e.project_id=p.id),
               (select count(*) from app.requirement_coverage c where c.project_id=p.id and c.status='covered'),
               (select count(*) from app.deliverable_sources ds where ds.project_id=p.id and ds.source_kind='model_run')
             from app.projects p where p.public_id=$1",
        ).bind(project.public_id).fetch_one(&mut *tx).await?;
        ensure!(
            deliverables == 1 && evidence == 0 && covered == 0 && linked_runs == 2,
            "coverage failure created a partial deliverable or model assessment became a proof"
        );
        tx.commit().await?;
    }
    Ok(())
}
