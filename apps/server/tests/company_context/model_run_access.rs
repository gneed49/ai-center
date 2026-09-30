//! No provider calls: loss of membership while a deterministic engine is paused.
use super::*;
use ai_center_server::{
    agent::{
        AgentEngine, AgentInput, ContextSelectionDraft, ContextSelectionInput,
        CoverageEvaluationDraft, CoverageEvaluationInput, EngineOutput, StewardInput,
        StewardOutput, TechnicalPlanDraft, TechnicalPlanInput,
    },
    artifacts::generation_contract::{ArtifactDraft, ArtifactGenerationInput, GenerateArtifact},
    error::AppResult,
    models::{AgentTurn, SendMessage},
    steward::{self, StewardConfig},
};
use async_trait::async_trait;
use std::time::Duration;

#[derive(Default)]
struct PausedEngine {
    started: tokio::sync::Notify,
}
impl PausedEngine {
    async fn pause(&self) {
        self.started.notify_one();
        std::future::pending::<()>().await;
    }
}
#[async_trait]
impl AgentEngine for PausedEngine {
    fn provider_name(&self) -> &'static str {
        "deterministic"
    }
    fn requested_model(&self) -> &'static str {
        "[FICTIF] access-loss"
    }
    async fn respond(&self, input: AgentInput) -> AppResult<EngineOutput<AgentTurn>> {
        self.pause().await;
        DeterministicEngine.respond(input).await
    }
    async fn generate_artifact(
        &self,
        input: ArtifactGenerationInput,
    ) -> AppResult<EngineOutput<ArtifactDraft>> {
        self.pause().await;
        DeterministicEngine.generate_artifact(input).await
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
        self.pause().await;
        DeterministicEngine.analyze_contradictions(input).await
    }
}
async fn scoped_tx(state: &AppState) -> Result<sqlx::Transaction<'_, sqlx::Postgres>> {
    let mut tx = state.pool.begin().await?;
    sqlx::query("select set_config('app.current_actor_id',$1,true),set_config('app.current_workspace_id',$2,true),set_config('app.current_workspace_role',$3,true)")
        .bind(state.actor_id.to_string()).bind(state.workspace_internal_id.context("workspace")?.to_string())
        .bind(&state.workspace_role).execute(&mut *tx).await?;
    Ok(tx)
}
async fn cancel(state: &AppState, run: i64) -> Result<bool> {
    let mut tx = scoped_tx(state).await?;
    let result = sqlx::query_scalar("select app.cancel_model_run_after_access_loss($1)")
        .bind(run)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(result)
}
async fn insert_run(state: &AppState, project: Uuid) -> Result<i64> {
    let mut tx = scoped_tx(state).await?;
    let id = sqlx::query_scalar("insert into app.model_runs(workspace_id,project_id,operation,provider,model,prompt_version,schema_version,input_hash,status,started_at)
        select workspace_id,id,'extract_knowledge','deterministic','[FICTIF] run','fixture','fixture',repeat('0',64),'running',clock_timestamp() from app.projects where public_id=$1 returning id")
        .bind(project).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(id)
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Twelve combinations share the real authorization/failure pipeline.
async fn changed_authorization_cancels_chat_artifact_and_steward_runs_without_output() -> Result<()>
{
    for operation in ["chat", "artifact", "steward"] {
        let actor = isolated_actor().await?;
        for (initial_role, next_role, invitation_status) in [
            ("editor", Some("viewer"), None),
            ("editor", None, Some("revoked")),
            ("owner", Some("editor"), None),
            ("editor", Some("owner"), None),
        ] {
            let owner = create(&actor, Uuid::new_v4()).await?;
            let editor_actor = Uuid::new_v4();
            let member = add_member(&owner, editor_actor, initial_role).await?;
            let (project, session) = ready_project(&owner, "[FICTIF] Perte d’accès").await?;
            if operation == "steward" {
                let company_scope = company::overview(&owner)
                    .await?
                    .company_scope
                    .context("company")?
                    .project_public_id;
                fixture_knowledge(
                    &owner,
                    company_scope,
                    "business_rule",
                    "[FICTIF] Validation humaine traçable reste obligatoire.",
                )
                .await?;
            }
            let mut editor = select(&owner, owner.workspace_id, editor_actor).await?;
            let engine = Arc::new(PausedEngine::default());
            editor.engine = engine.clone();
            let request = tokio::spawn(async move {
                if operation == "steward" {
                    steward::analyze_project(&editor, project, StewardConfig::default())
                        .await
                        .map(|_| ())
                } else if operation == "artifact" {
                    service::artifact_generation::generate(
                        &editor,
                        project,
                        GenerateArtifact {
                            session_id: session,
                            artifact_type: "kickoff".into(),
                            instructions: "[FICTIF] Préparer le lancement".into(),
                        },
                        None,
                    )
                    .await
                    .map(|_| ())
                } else {
                    service::send_message(
                        &editor,
                        session,
                        SendMessage {
                            content: "[FICTIF] Préparer le lancement".into(),
                            client_message_id: Uuid::new_v4(),
                        },
                    )
                    .await
                    .map(|_| ())
                }
            });
            tokio::time::timeout(Duration::from_secs(10), engine.started.notified()).await?;
            company::update_member(
                &owner,
                member,
                UpdateMember {
                    role: next_role.map(str::to_owned),
                    invitation_status: invitation_status.map(str::to_owned),
                },
                None,
            )
            .await?;
            assert!(matches!(
                tokio::time::timeout(Duration::from_secs(10), request).await??,
                Err(AppError::Forbidden)
            ));
            let mut tx = scoped_tx(&owner).await?;
            let (status,code,output,starter,terminal):(String,Option<String>,Option<serde_json::Value>,Option<Uuid>,bool)=sqlx::query_as(
                "select status,error_class,output,started_by_actor_id,completed_at is not null from app.model_runs
                 where project_id=(select id from app.projects where public_id=$1) order by id desc limit 1")
                .bind(project).fetch_one(&mut *tx).await?;
            assert_eq!(status, "cancelled");
            assert_eq!(code.as_deref(), Some("access_changed"));
            assert!(output.is_none());
            assert_eq!(starter, Some(editor_actor));
            assert!(terminal);
            let artifacts: i64=sqlx::query_scalar("select count(*) from app.artifact_documents where project_id=(select id from app.projects where public_id=$1)")
                .bind(project).fetch_one(&mut *tx).await?;
            let replies: i64=sqlx::query_scalar("select count(*) from app.messages where session_id=(select id from app.sessions where public_id=$1) and role='assistant'")
                .bind(session).fetch_one(&mut *tx).await?;
            assert_eq!(artifacts, 0);
            assert_eq!(replies, 0);
            tx.commit().await?;
            if operation == "steward" {
                let mut tx = scoped_tx(&owner).await?;
                let (status,seconds):(String,f64)=sqlx::query_as("select status,extract(epoch from (lease_until-clock_timestamp()))::double precision from app.steward_scan_progress where workspace_id=app.current_workspace_id()")
                    .fetch_one(&mut *tx).await?;
                assert_eq!(status, "running");
                assert!(
                    seconds > 0.0 && seconds <= 660.0,
                    "revoked actor cannot release a lease, but the wait is bounded"
                );
                // Simulate elapsed lease time; an authorized actor can recover.
                sqlx::query("update app.steward_scan_progress set lease_until=clock_timestamp()-interval '1 second' where workspace_id=app.current_workspace_id()")
                    .execute(&mut *tx).await?;
                tx.commit().await?;
                steward::analyze_project(&owner, project, StewardConfig::default()).await?;
                let mut tx = scoped_tx(&owner).await?;
                let running:bool=sqlx::query_scalar("select status='running' from app.steward_scan_progress where workspace_id=app.current_workspace_id()")
                    .fetch_one(&mut *tx).await?;
                assert!(!running);
                tx.commit().await?;
            }
        }
    }
    Ok(())
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Ownership immutability plus all narrow-helper refusal cases.
async fn access_loss_helper_cannot_cancel_other_actors_scopes_completed_or_legacy_runs()
-> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let foreign = create(&actor, Uuid::new_v4()).await?;
    let (project, _) = ready_project(&owner, "[FICTIF] Autorité de clôture").await?;
    let editor_actor = Uuid::new_v4();
    let member = add_member(&owner, editor_actor, "editor").await?;
    let editor = select(&owner, owner.workspace_id, editor_actor).await?;
    let running = insert_run(&editor, project).await?;
    let completed = insert_run(&editor, project).await?;
    let other_actor_run = insert_run(&owner, project).await?;
    assert!(
        !cancel(&editor, running).await?,
        "an active writer cannot use fallback cancellation"
    );
    let mut tx = scoped_tx(&owner).await?;
    assert!(
        sqlx::query(
            "update app.model_runs set started_by_actor_id=app.current_actor_id() where id=$1"
        )
        .bind(running)
        .execute(&mut *tx)
        .await
        .is_err()
    );
    tx.rollback().await?;
    let mut tx = scoped_tx(&owner).await?;
    assert!(sqlx::query("insert into app.model_runs(workspace_id,project_id,operation,provider,model,prompt_version,schema_version,input_hash,started_by_actor_id)
        select workspace_id,id,'extract_knowledge','deterministic','[FICTIF] forged','fixture','fixture',repeat('0',64),$2 from app.projects where public_id=$1")
        .bind(project).bind(editor_actor).execute(&mut *tx).await.is_err());
    tx.rollback().await?;
    let mut tx = scoped_tx(&editor).await?;
    sqlx::query("update app.model_runs set status='completed',output='{}'::jsonb,completed_at=clock_timestamp() where id=$1")
        .bind(completed).execute(&mut *tx).await?;
    tx.commit().await?;

    // Seed only a historical NULL-owner row through the guarded operator URL;
    // normal runtime INSERT must never be able to fabricate such ownership.
    let raw = std::env::var("AI_CENTER_ADMIN_DATABASE_URL").context("guarded admin URL")?;
    let url = url::Url::parse(&raw).map_err(|_| anyhow::anyhow!("invalid guarded admin URL"))?;
    ensure!(
        url.scheme() == "postgresql"
            && url.host_str() == Some("127.0.0.1")
            && url.port() == Some(55322)
            && url.username() == "postgres"
            && url.path() == "/postgres"
            && url.query().is_none()
            && url.fragment().is_none(),
        "guarded operator database required"
    );
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&raw)
        .await?;
    let legacy: i64=sqlx::query_scalar("insert into app.model_runs(workspace_id,project_id,operation,provider,model,prompt_version,schema_version,input_hash,status,started_at,started_by_actor_id)
        select workspace_id,id,'extract_knowledge','deterministic','[FICTIF] historical','fixture','fixture',repeat('0',64),'running',clock_timestamp(),null from app.projects where public_id=$1 returning id")
        .bind(project).fetch_one(&admin).await?;
    admin.close().await;

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
    let viewer = select(&owner, owner.workspace_id, editor_actor).await?;
    assert!(!cancel(&viewer, other_actor_run).await?);
    assert!(!cancel(&viewer, completed).await?);
    assert!(!cancel(&viewer, legacy).await?);
    assert!(!cancel(&viewer, i64::MAX).await?);
    add_member(&foreign, editor_actor, "viewer").await?;
    let other_scope = select(&foreign, foreign.workspace_id, editor_actor).await?;
    assert!(!cancel(&other_scope, running).await?);
    assert!(cancel(&viewer, running).await?);
    assert!(
        !cancel(&viewer, running).await?,
        "a terminal cancellation is not replayed as a new write"
    );
    let mut tx = scoped_tx(&owner).await?;
    let rows: Vec<(i64, String, Option<Uuid>)> = sqlx::query_as(
        "select id,status,started_by_actor_id from app.model_runs where id=any($1) order by id",
    )
    .bind(vec![running, completed, other_actor_run, legacy])
    .fetch_all(&mut *tx)
    .await?;
    assert_eq!(rows[0], (running, "cancelled".into(), Some(editor_actor)));
    assert_eq!(rows[1], (completed, "completed".into(), Some(editor_actor)));
    assert_eq!(
        rows[2],
        (other_actor_run, "running".into(), Some(owner.actor_id))
    );
    assert_eq!(rows[3], (legacy, "running".into(), None));
    tx.commit().await?;
    Ok(())
}
