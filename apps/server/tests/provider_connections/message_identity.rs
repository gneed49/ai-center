//! Message identity is durable across distinct HTTP command identities.
use super::*;
use ai_center_server::{
    agent::DeterministicEngine,
    error::{ProviderError, ProviderErrorClass},
    models::{CreateProject, CreateSession},
    service,
};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

struct RecordingEngine {
    fail_next: AtomicBool,
    messages: Mutex<Vec<String>>,
}

#[async_trait::async_trait]
impl AgentEngine for RecordingEngine {
    fn provider_name(&self) -> &'static str {
        "recording-fixture"
    }
    fn requested_model(&self) -> &'static str {
        "recording-model"
    }
    async fn respond(&self, input: AgentInput) -> AppResult<EngineOutput<AgentTurn>> {
        self.messages
            .lock()
            .unwrap()
            .push(input.user_message.clone());
        if self.fail_next.swap(false, Ordering::SeqCst) {
            return Err(ProviderError::new(
                "recording-fixture",
                ProviderErrorClass::Quota,
                1,
                None,
            )
            .into());
        }
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

async fn counts(admin: &PgPool, session: Uuid) -> Result<(i64, i64, i64)> {
    Ok(sqlx::query_as(
        "select (select count(*) from app.messages where session_id=s.id),
                (select count(*) from app.model_runs where session_id=s.id),
                (select count(*) from app.mutation_proposals where session_id=s.id)
         from app.sessions s where public_id=$1",
    )
    .bind(session)
    .fetch_one(admin)
    .await?)
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn changed_content_is_rejected_after_failure_or_success_without_new_work() -> Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(6)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;
    let admin = PgPoolOptions::new()
        .max_connections(2)
        .connect(&std::env::var("AI_CENTER_ADMIN_DATABASE_URL")?)
        .await?;
    let runtime_role: (String, bool) = sqlx::query_as(
        "select current_user::text,rolbypassrls or rolsuper from pg_roles where rolname=current_user"
    ).fetch_one(&pool).await?;
    ensure!(runtime_role == ("ai_center_runtime".into(), false));

    for initially_fails in [true, false] {
        let workspace = Uuid::new_v4();
        let actor = Uuid::new_v4();
        let internal_id: i64 = sqlx::query_scalar(
            "insert into app.workspaces(public_id,owner_actor_id,name) values($1,$2,'Message identity fixture') returning id"
        ).bind(workspace).bind(actor).fetch_one(&admin).await?;
        sqlx::query("insert into app.workspace_members(workspace_id,actor_id,role,invitation_status,invited_by_actor_id,accepted_at) values($1,$2,'owner','accepted',$2,now())")
            .bind(internal_id).bind(actor).execute(&admin).await?;
        let engine = Arc::new(RecordingEngine {
            fail_next: AtomicBool::new(initially_fails),
            messages: Mutex::new(Vec::new()),
        });
        let mut scoped = state(&pool, workspace, actor, runtime()?).await?;
        scoped.engine = engine.clone();
        let project = service::create_project(
            &scoped,
            CreateProject {
                name: "Stable intention".into(),
                objective: "Keep the original message".into(),
            },
        )
        .await?;
        let session = service::create_session(
            &scoped,
            project.public_id,
            CreateSession {
                node_key: "product".into(),
                title: None,
            },
        )
        .await?
        .session
        .public_id;
        let (url, task) = server(scoped).await?;
        let endpoint = format!(
            "{url}/api/projects/{}/sessions/{session}/messages",
            project.public_id
        );
        let client = reqwest::Client::new();
        let message = Uuid::new_v4();
        let original = "Les décisions confirmées doivent être conservées.";
        let response = client
            .post(&endpoint)
            .header("Idempotency-Key", Uuid::new_v4().to_string())
            .json(&json!({"client_message_id":message,"content":format!("  {original}\n")}))
            .send()
            .await?;
        ensure!(response.status().is_success() != initially_fails);
        ensure!(engine.messages.lock().unwrap().as_slice() == [original]);
        let before = counts(&admin, session).await?;

        // A new command is not permission to change the retained message.
        let mismatch = Uuid::new_v4();
        for _ in 0..2 {
            let response = client.post(&endpoint).header("Idempotency-Key", mismatch.to_string())
                .json(&json!({"client_message_id":message,"content":"Supprimer toutes les décisions confirmées."}))
                .send().await?;
            ensure!(
                response.status() == reqwest::StatusCode::CONFLICT,
                "changed message content must return 409 after failure or success"
            );
        }
        ensure!(
            counts(&admin, session).await? == before,
            "mismatch created messages, runs or proposals"
        );
        ensure!(
            engine.messages.lock().unwrap().as_slice() == [original],
            "mismatch reached the engine"
        );

        let response = client
            .post(&endpoint)
            .header("Idempotency-Key", Uuid::new_v4().to_string())
            .json(&json!({"client_message_id":message,"content":format!("\n{original}  ")}))
            .send()
            .await?;
        ensure!(
            response.status().is_success(),
            "same normalized content must resume or replay"
        );
        let after = counts(&admin, session).await?;
        ensure!(after.0 == 2 && after.1 == if initially_fails { 2 } else { 1 });
        if !initially_fails {
            ensure!(after == before, "successful replay changed projections");
        }
        ensure!(engine.messages.lock().unwrap().len() == if initially_fails { 2 } else { 1 });
        let stored: String = sqlx::query_scalar(
            "select content from app.messages where client_message_id=$1 and role='user'",
        )
        .bind(message)
        .fetch_one(&admin)
        .await?;
        ensure!(stored == original);
        task.abort();
    }
    Ok(())
}
