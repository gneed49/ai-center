//! Real PostgreSQL/RLS and HTTP contracts, without browser or provider calls.
use ai_center_server::{
    agent::{
        AgentEngine, AgentInput, ContextSelectionDraft, ContextSelectionInput,
        CoverageEvaluationDraft, CoverageEvaluationInput, EngineOutput, StewardInput,
        StewardOutput, TechnicalPlanDraft, TechnicalPlanInput,
    },
    auth::AuthRuntime,
    config::AuthMode,
    error::{AppError, AppResult},
    models::AgentTurn,
    providers::{
        self, CreateConnection, ProviderRuntime, Selection, UpdateConnection,
        encryption::CredentialCipher,
    },
    routes,
    service::AppState,
};
use anyhow::{Result, ensure};
use ring::rand::{SecureRandom, SystemRandom};
use secrecy::SecretString;
use serde_json::{Value, json};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::Arc;
use uuid::Uuid;

// A non-network failing default proves that personal choices are honored even
// when the application's default cannot produce any business output.
struct ForbiddenDefault;
#[async_trait::async_trait]
impl AgentEngine for ForbiddenDefault {
    fn provider_name(&self) -> &'static str {
        "forbidden-default"
    }
    fn requested_model(&self) -> &'static str {
        "forbidden-default"
    }
    async fn respond(&self, _: AgentInput) -> AppResult<EngineOutput<AgentTurn>> {
        Err(AppError::Agent("default must not be used".into()))
    }
    async fn select_context(
        &self,
        _: ContextSelectionInput,
    ) -> AppResult<EngineOutput<ContextSelectionDraft>> {
        Err(AppError::Agent("default must not be used".into()))
    }
    async fn generate_technical_plan(
        &self,
        _: TechnicalPlanInput,
    ) -> AppResult<EngineOutput<TechnicalPlanDraft>> {
        Err(AppError::Agent("default must not be used".into()))
    }
    async fn evaluate_coverage(
        &self,
        _: CoverageEvaluationInput,
    ) -> AppResult<EngineOutput<CoverageEvaluationDraft>> {
        Err(AppError::Agent("default must not be used".into()))
    }
    async fn analyze_contradictions(
        &self,
        _: StewardInput,
    ) -> AppResult<EngineOutput<StewardOutput>> {
        Err(AppError::Agent("default must not be used".into()))
    }
}

async fn state(
    pool: &PgPool,
    workspace_id: Uuid,
    actor_id: Uuid,
    runtime: Arc<ProviderRuntime>,
) -> Result<AppState> {
    let (id, role): (i64, String) =
        sqlx::query_as("select workspace_id,role from app.authorize_workspace_member($1,$2)")
            .bind(workspace_id)
            .bind(actor_id)
            .fetch_one(pool)
            .await?;
    Ok(AppState {
        pool: pool.clone(),
        engine: Arc::new(ForbiddenDefault),
        providers: Some(runtime),
        workspace_id,
        workspace_internal_id: Some(id),
        workspace_role: role,
        actor_id,
        agent_mode: "deterministic",
        steward_trigger: None,
    })
}
fn runtime() -> Result<Arc<ProviderRuntime>> {
    let mut key = [0; 32];
    SystemRandom::new()
        .fill(&mut key)
        .map_err(|_| anyhow::anyhow!("entropy unavailable"))?;
    Ok(Arc::new(ProviderRuntime {
        cipher: Some(CredentialCipher::from_bytes(&key)?),
        subscriptions: None,
    }))
}
async fn context(
    pool: &PgPool,
    state: &AppState,
) -> Result<sqlx::Transaction<'static, sqlx::Postgres>> {
    let mut tx = pool.begin().await?;
    sqlx::query("select set_config('app.current_workspace_id',$1,true),set_config('app.current_actor_id',$2,true),set_config('app.current_workspace_role',$3,true)").bind(state.workspace_internal_id.unwrap().to_string()).bind(state.actor_id.to_string()).bind(&state.workspace_role).execute(&mut *tx).await?;
    Ok(tx)
}
async fn server(state: AppState) -> Result<(String, tokio::task::JoinHandle<()>)> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}", listener.local_addr()?);
    let auth = Arc::new(AuthRuntime::new(
        AuthMode::Local,
        state.workspace_id,
        state.actor_id,
        None,
    )?);
    let app = routes::router(Arc::new(state), auth, None, vec![]);
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Ok((url, task))
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn private_credentials_http_idempotency_selection_and_rls() -> Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;
    let admin = PgPoolOptions::new()
        .max_connections(2)
        .connect(&std::env::var("AI_CENTER_ADMIN_DATABASE_URL")?)
        .await?;
    let bypass: bool = sqlx::query_scalar(
        "select rolbypassrls or rolsuper from pg_roles where rolname=current_user",
    )
    .fetch_one(&pool)
    .await?;
    ensure!(!bypass, "RLS proof requires an unprivileged runtime");
    let owner = Uuid::new_v4();
    let editor = Uuid::new_v4();
    let viewer = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    let foreign_workspace = Uuid::new_v4();
    for public_id in [workspace, foreign_workspace] {
        let id:i64=sqlx::query_scalar("insert into app.workspaces(public_id,owner_actor_id,name) values($1,$2,'Provider integration test') returning id").bind(public_id).bind(owner).fetch_one(&admin).await?;
        for (actor, role) in [(owner, "owner"), (editor, "editor"), (viewer, "viewer")] {
            sqlx::query("insert into app.workspace_members(workspace_id,actor_id,role,invitation_status,invited_by_actor_id,accepted_at) values($1,$2,$3,'accepted',$2,now())").bind(id).bind(actor).bind(role).execute(&admin).await?;
        }
    }
    let shared = runtime()?;
    let owner_state = state(&pool, workspace, owner, shared.clone()).await?;
    let editor_state = state(&pool, workspace, editor, shared.clone()).await?;
    let viewer_state = state(&pool, workspace, viewer, shared.clone()).await?;
    let other_workspace = state(&pool, foreign_workspace, owner, shared).await?;
    let (url, http) = server(owner_state.clone()).await?;
    let (viewer_url, viewer_http) = server(viewer_state.clone()).await?;
    let client = reqwest::Client::new();
    let id = Uuid::new_v4();
    let command = Uuid::new_v4();
    let key = Uuid::new_v4().to_string();
    let settings_response = client.get(format!("{url}/api/ai/settings")).send().await?;
    ensure!(
        settings_response
            .headers()
            .get("cache-control")
            .is_some_and(|value| value == "no-store"),
        "private settings response must forbid caching"
    );
    let rejected_response = client
        .get(format!("{viewer_url}/api/ai/settings"))
        .send()
        .await?;
    ensure!(
        rejected_response.status().as_u16() == 403
            && rejected_response
                .headers()
                .get("cache-control")
                .is_some_and(|value| value == "no-store"),
        "private errors must forbid caching"
    );
    let body = json!({"id":id,"provider":"openai","name":"Personal OpenAI","model":"test-model-first","api_key":key});
    ensure!(
        client
            .post(format!("{url}/api/ai/connections"))
            .json(&body)
            .send()
            .await?
            .status()
            .as_u16()
            == 422,
        "mutation must require UUID idempotency key"
    );
    let response = client
        .post(format!("{url}/api/ai/connections"))
        .header("Idempotency-Key", command.to_string())
        .json(&body)
        .send()
        .await?;
    ensure!(
        response.status().is_success(),
        "create HTTP failed: {}",
        response.status()
    );
    let response: Value = response.json().await?;
    ensure!(
        !response.to_string().contains(&key)
            && response.get("api_key").is_none()
            && response.get("key_ciphertext").is_none(),
        "credential response must be redacted"
    );
    let replay = client
        .post(format!("{url}/api/ai/connections"))
        .header("Idempotency-Key", command.to_string())
        .json(&body)
        .send()
        .await?;
    ensure!(replay.status().is_success(), "idempotent replay failed");
    let mut changed = body.clone();
    changed["api_key"] = json!(Uuid::new_v4().to_string());
    ensure!(
        client
            .post(format!("{url}/api/ai/connections"))
            .header("Idempotency-Key", command.to_string())
            .json(&changed)
            .send()
            .await?
            .status()
            .as_u16()
            == 409,
        "a different key must not replay a previous mutation"
    );
    ensure!(
        client
            .get(format!("{viewer_url}/api/ai/settings"))
            .send()
            .await?
            .status()
            .as_u16()
            == 403,
        "viewer cannot read settings"
    );
    ensure!(
        client
            .post(format!("{viewer_url}/api/ai/connections"))
            .header("Idempotency-Key", Uuid::new_v4().to_string())
            .json(&body)
            .send()
            .await?
            .status()
            .as_u16()
            == 403,
        "viewer cannot mutate settings"
    );
    let encrypted: Vec<u8> = sqlx::query_scalar(
        "select key_ciphertext from app.provider_connections where public_id=$1",
    )
    .bind(id)
    .fetch_one(&admin)
    .await?;
    ensure!(
        !encrypted
            .windows(key.len())
            .any(|part| part == key.as_bytes()),
        "database must not contain plaintext"
    );
    let second = providers::create(
        &owner_state,
        CreateConnection {
            id: Uuid::new_v4(),
            provider: "openai".into(),
            name: "Second account".into(),
            model: "test-model-second".into(),
            api_key: Some(SecretString::from(Uuid::new_v4().to_string())),
        },
        None,
    )
    .await?;
    ensure!(
        providers::settings(&owner_state).await?.connections.len() == 2,
        "multiple keys per provider required"
    );
    ensure!(
        providers::settings(&editor_state)
            .await?
            .connections
            .is_empty(),
        "same-workspace peer sees private metadata"
    );
    ensure!(
        providers::settings(&other_workspace)
            .await?
            .connections
            .is_empty(),
        "same actor sees another workspace's key"
    );
    for forbidden in [&editor_state, &other_workspace] {
        ensure!(
            providers::update(
                forbidden,
                id,
                UpdateConnection {
                    name: "Hijack".into(),
                    model: "other".into(),
                    api_key: None
                },
                None
            )
            .await
            .is_err()
        );
        ensure!(
            providers::select(
                forbidden,
                Selection {
                    mode: "connection".into(),
                    connection_id: Some(id)
                },
                None
            )
            .await
            .is_err()
        );
        let mut tx = context(&pool, forbidden).await?;
        let count: i64 =
            sqlx::query_scalar("select count(*) from app.provider_connections where public_id=$1")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        ensure!(count == 0, "FORCE RLS did not hide the ciphertext row");
        tx.rollback().await?;
    }
    let mut tx = context(&pool, &editor_state).await?;
    let forged=sqlx::query("insert into app.provider_selections(workspace_id,actor_id,mode) values($1,$2,'deterministic')").bind(owner_state.workspace_internal_id).bind(owner).execute(&mut *tx).await;
    ensure!(forged.is_err(), "RLS must reject ownership forgery");
    tx.rollback().await?;
    providers::select(
        &owner_state,
        Selection {
            mode: "connection".into(),
            connection_id: Some(id),
        },
        None,
    )
    .await?;
    let fixed = providers::resolve_engine(&owner_state).await?;
    ensure!(fixed.requested_model() == "test-model-first");
    providers::update(
        &owner_state,
        id,
        UpdateConnection {
            name: "Renamed".into(),
            model: "test-model-updated".into(),
            api_key: None,
        },
        None,
    )
    .await?;
    let retained: Vec<u8> = sqlx::query_scalar(
        "select key_ciphertext from app.provider_connections where public_id=$1",
    )
    .bind(id)
    .fetch_one(&admin)
    .await?;
    ensure!(retained == encrypted, "omitting a key must retain it");
    ensure!(
        fixed.requested_model() == "test-model-first",
        "an operation's engine identity changed"
    );
    ensure!(
        providers::resolve_engine(&owner_state)
            .await?
            .requested_model()
            == "test-model-updated",
        "next operation did not load the current selection"
    );
    providers::update(
        &owner_state,
        id,
        UpdateConnection {
            name: "Rotated".into(),
            model: "test-model-updated".into(),
            api_key: Some(SecretString::from(Uuid::new_v4().to_string())),
        },
        None,
    )
    .await?;
    let rotated: Vec<u8> = sqlx::query_scalar(
        "select key_ciphertext from app.provider_connections where public_id=$1",
    )
    .bind(id)
    .fetch_one(&admin)
    .await?;
    ensure!(
        rotated != encrypted,
        "replacing a key must rotate its envelope"
    );
    let mut unavailable = owner_state.clone();
    unavailable.providers = Some(Arc::new(ProviderRuntime {
        cipher: None,
        subscriptions: None,
    }));
    ensure!(
        providers::resolve_engine(&unavailable).await.is_err(),
        "selected unavailable key silently fell back"
    );
    sqlx::query("update app.provider_connections set key_ciphertext=decode(repeat('00',40),'hex') where public_id=$1").bind(id).execute(&admin).await?;
    ensure!(
        providers::resolve_engine(&owner_state).await.is_err(),
        "corrupt credentials silently fell back"
    );
    let record:String=sqlx::query_scalar("select row_to_json(record)::text from app.idempotency_records record where idempotency_key=$1 and actor_id=$2").bind(command.to_string()).bind(owner).fetch_one(&admin).await?;
    ensure!(
        !record.contains(&key) && !record.contains("key_ciphertext") && !record.contains("api_key"),
        "idempotency stored credential material"
    );
    providers::delete(&owner_state, id, None).await?;
    let settings = providers::settings(&owner_state).await?;
    ensure!(
        settings.selection.mode == "deterministic" && settings.selection.connection_id.is_none(),
        "deletion did not atomically return to deterministic"
    );
    ensure!(
        providers::resolve_engine(&owner_state)
            .await?
            .provider_name()
            == "deterministic"
    );
    providers::delete(&owner_state, second.id, None).await?;
    // Selected deterministic actually overrides the application's default for a
    // business operation, and the chosen identity is recorded in model_runs.
    let project = ai_center_server::service::create_project(
        &owner_state,
        ai_center_server::models::CreateProject {
            name: "Provider identity".into(),
            objective: "Test isolation".into(),
        },
    )
    .await?;
    let session = ai_center_server::service::create_session(
        &owner_state,
        project.public_id,
        ai_center_server::models::CreateSession {
            node_key: "product".into(),
            title: None,
        },
    )
    .await?;
    let turn = ai_center_server::service::send_message(
        &owner_state,
        session.session.public_id,
        ai_center_server::models::SendMessage {
            client_message_id: Uuid::new_v4(),
            content: "Nous devons conserver les décisions confirmées.".into(),
        },
    )
    .await?;
    let provider: String = sqlx::query_scalar(
        "select provider from app.model_runs where workspace_id=$1 order by id desc limit 1",
    )
    .bind(owner_state.workspace_internal_id)
    .fetch_one(&admin)
    .await?;
    ensure!(
        provider == "deterministic",
        "model run did not record selected engine"
    );
    // The scanner is another accepted member with an unusable personal key.
    // Its default/credential must never replace the event author's selection.
    let bad = providers::create(
        &editor_state,
        CreateConnection {
            id: Uuid::new_v4(),
            provider: "openai".into(),
            name: "Scanner account".into(),
            model: "never-contacted".into(),
            api_key: Some(SecretString::from(Uuid::new_v4().to_string())),
        },
        None,
    )
    .await?;
    providers::select(
        &editor_state,
        Selection {
            mode: "connection".into(),
            connection_id: Some(bad.id),
        },
        None,
    )
    .await?;
    sqlx::query("update app.provider_connections set key_ciphertext=decode(repeat('00',40),'hex') where public_id=$1").bind(bad.id).execute(&admin).await?;
    ai_center_server::service::decide_proposals(
        &owner_state,
        session.session.public_id,
        ai_center_server::models::DecideProposals {
            proposal_ids: turn
                .proposals
                .iter()
                .map(|proposal| proposal.public_id)
                .collect(),
            decision: ai_center_server::models::ProposalDecision::Confirm,
        },
    )
    .await?;
    let originated: bool = sqlx::query_scalar("select bool_and(requested_by_actor_id=$1) from app.domain_events where workspace_id=$2 and event_type='knowledge.committed'").bind(owner).bind(owner_state.workspace_internal_id).fetch_one(&admin).await?;
    ensure!(originated, "outbox failed to persist the requesting actor");
    let drain = ai_center_server::steward::drain_steward_outbox(&editor_state).await?;
    ensure!(
        drain.failed == 0 && drain.processed > 0,
        "scanner must use original actor's deterministic selection"
    );
    let provider: String=sqlx::query_scalar("select provider from app.model_runs where workspace_id=$1 and operation='assess_contradiction' order by id desc limit 1").bind(owner_state.workspace_internal_id).fetch_one(&admin).await?;
    ensure!(
        provider == "deterministic",
        "Steward used the scanner's provider"
    );
    // A later event from that unusable editor must fail even when the drain
    // runs under an owner with a working deterministic engine.
    sqlx::query("insert into app.domain_events(workspace_id,project_id,requested_by_actor_id,event_type,aggregate_kind,aggregate_public_id) select workspace_id,id,$2,'knowledge.revised','project',public_id from app.projects where public_id=$1").bind(project.public_id).bind(editor).execute(&admin).await?;
    let drain = ai_center_server::steward::drain_steward_outbox(&owner_state).await?;
    ensure!(
        drain.failed == 1,
        "Steward silently substituted the owner's working engine"
    );
    http.abort();
    viewer_http.abort();
    // The owning disposable stack removes fixture volumes. Audit events are
    // immutable, so tests intentionally do not cascade-delete workspaces.
    Ok(())
}
