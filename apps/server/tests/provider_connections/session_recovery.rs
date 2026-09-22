//! Reload recovery and teammate attribution use local engine doubles only.
use super::message_identity::RecordingEngine;
use super::*;
use ai_center_server::{
    error::ProviderErrorClass,
    models::{CreateProject, CreateSession},
    service,
};
use anyhow::Context;
use std::sync::{Mutex, atomic::AtomicBool};

pub(super) fn guarded_urls() -> Result<(String, String)> {
    let raw = std::env::var("DATABASE_URL")?;
    let admin = std::env::var("AI_CENTER_ADMIN_DATABASE_URL")?;
    for (input, role) in [(&raw, "ai_center_runtime"), (&admin, "postgres")] {
        let url =
            url::Url::parse(input).map_err(|_| anyhow::anyhow!("invalid isolated database URL"))?;
        ensure!(
            url.scheme() == "postgresql"
                && url.host_str() == Some("127.0.0.1")
                && url.port() == Some(55322)
                && url.username() == role
                && url.path() == "/postgres"
                && url.query().is_none()
                && url.fragment().is_none(),
            "guarded isolated database on 55322 required"
        );
    }
    ensure!(
        std::env::var("AI_CENTER_EXPECT_DATABASE_ROLE").as_deref() == Ok("ai_center_runtime")
            && std::env::var("AI_CENTER_AGENT_MODE").as_deref() == Ok("deterministic"),
        "explicit isolated deterministic mode required"
    );
    Ok((raw, admin))
}

async fn fixture() -> Result<(AppState, AppState, Arc<RecordingEngine>, PgPool, Uuid, Uuid)> {
    let (raw, admin_url) = guarded_urls()?;
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&raw)
        .await?;
    let admin = PgPoolOptions::new()
        .max_connections(2)
        .connect(&admin_url)
        .await?;
    let posture:(String,bool)=sqlx::query_as("select current_user::text,rolsuper or rolbypassrls from pg_roles where rolname=current_user").fetch_one(&pool).await?;
    ensure!(
        posture == ("ai_center_runtime".into(), false),
        "runtime RLS required"
    );
    let workspace = Uuid::new_v4();
    let owner = Uuid::new_v4();
    let editor = Uuid::new_v4();
    let id:i64=sqlx::query_scalar("insert into app.workspaces(public_id,owner_actor_id,name) values($1,$2,'[FICTIF] Reprise équipe') returning id").bind(workspace).bind(owner).fetch_one(&admin).await?;
    for (actor, role, name) in [
        (owner, "owner", "[FICTIF] Camille"),
        (editor, "editor", "[FICTIF] Alex"),
    ] {
        sqlx::query("insert into app.workspace_members(workspace_id,actor_id,role,invitation_status,accepted_at,display_name) values($1,$2,$3,'accepted',now(),$4)").bind(id).bind(actor).bind(role).bind(name).execute(&admin).await?;
    }
    let engine = Arc::new(RecordingEngine {
        fail_next: AtomicBool::new(true),
        messages: Mutex::new(Vec::new()),
        failure_class: ProviderErrorClass::Timeout,
    });
    let shared = runtime()?;
    let mut owner = state(&pool, workspace, owner, shared.clone()).await?;
    let mut editor = state(&pool, workspace, editor, shared).await?;
    owner.engine = engine.clone();
    editor.engine = engine.clone();
    let project = service::create_project(
        &owner,
        CreateProject {
            name: "[FICTIF] Reprise".into(),
            objective: "Conserver le message exact et son auteur.".into(),
        },
    )
    .await?;
    let session = service::create_session(
        &owner,
        project.public_id,
        CreateSession {
            node_key: "product".into(),
            title: None,
        },
    )
    .await?;
    Ok((
        owner,
        editor,
        engine,
        admin,
        project.public_id,
        session.session.public_id,
    ))
}

async fn assert_forged_recovery_rejected(
    client: &reqwest::Client,
    destination: &str,
    body: &Value,
    expected: u16,
) -> Result<()> {
    let response = client
        .post(destination)
        .header("Idempotency-Key", Uuid::new_v4().to_string())
        .json(body)
        .send()
        .await?;
    ensure!(
        response.status().as_u16() == expected,
        "forged recovery was not refused"
    );
    Ok(())
}

#[tokio::test]
// Keep the failure, fresh-client read, teammate refusal and successful replay
// together: their shared call count is the regression's central assertion.
#[allow(clippy::too_many_lines)]
async fn reload_recovers_only_own_command_with_exact_identity_and_no_automatic_call() -> Result<()>
{
    let (owner, editor, engine, admin, project, session) = fixture().await?;
    let (url, owner_server) = server(owner.clone()).await?;
    let (other_url, editor_server) = server(editor).await?;
    let path = format!("/api/projects/{project}/sessions/{session}");
    let endpoint = format!("{url}{path}/messages");
    let original_key = Uuid::new_v4();
    let message_id = Uuid::new_v4();
    let submitted = "  [FICTIF] Les décisions doivent rester traçables.\n";
    let body = json!({"content":submitted,"client_message_id":message_id});
    let client = reqwest::Client::new();
    let failed = client
        .post(&endpoint)
        .header("Idempotency-Key", original_key.to_string())
        .json(&body)
        .send()
        .await?;
    ensure!(failed.status().as_u16() == 503);
    // A new client represents a reload: its command memory is empty.
    let reloaded = reqwest::Client::new();
    let view: Value = reloaded
        .get(format!("{url}{path}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    ensure!(
        engine.messages.lock().unwrap().len() == 1,
        "reading a receipt called the engine"
    );
    ensure!(
        view["messages"][0]["author_name"] == "[FICTIF] Camille"
            && view["messages"][0]["is_own"] == true
    );
    let command = &view["message_commands"][0];
    ensure!(command["status"] == "retryable" && command["can_retry"] == true);
    ensure!(
        command["idempotency_key"] == original_key.to_string()
            && command["client_message_id"] == message_id.to_string()
    );
    ensure!(
        command["submitted_content"] == submitted,
        "original whitespace was lost"
    );
    let colleague: Value = reloaded
        .get(format!("{other_url}{path}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    ensure!(
        colleague["message_commands"]
            .as_array()
            .context("commands")?
            .is_empty()
    );
    ensure!(
        colleague["messages"][0]["is_own"] == false
            && colleague["messages"][0]["author_name"] == "[FICTIF] Camille"
    );
    for (destination, expected) in [(&other_url, 403), (&url, 409)] {
        assert_forged_recovery_rejected(
            &reloaded,
            &format!("{destination}{path}/messages"),
            &body,
            expected,
        )
        .await?;
    }
    ensure!(engine.messages.lock().unwrap().len() == 1);
    let recovered_body = json!({"content":command["submitted_content"],"client_message_id":command["client_message_id"]});
    for _ in 0..2 {
        let response = reloaded
            .post(&endpoint)
            .header(
                "Idempotency-Key",
                command["idempotency_key"].as_str().context("key")?,
            )
            .json(&recovered_body)
            .send()
            .await?;
        ensure!(response.status().is_success());
    }
    ensure!(
        engine.messages.lock().unwrap().len() == 2,
        "successful replay repeated the provider call"
    );
    let final_view: Value = reloaded
        .get(format!("{url}{path}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    ensure!(final_view["messages"].as_array().context("messages")?.len() == 2);
    ensure!(
        final_view["message_commands"][0]["status"] == "completed"
            && final_view["message_commands"][0]["can_retry"] == false
    );
    owner_server.abort();
    editor_server.abort();
    owner.pool.close().await;
    admin.close().await;
    Ok(())
}

#[tokio::test]
async fn unfinished_legacy_expired_and_withdrawn_messages_never_invent_recovery_rights()
-> Result<()> {
    let (owner, _, engine, admin, project, session) = fixture().await?;
    let (url, server) = server(owner.clone()).await?;
    let path = format!("{url}/api/projects/{project}/sessions/{session}");
    let key = Uuid::new_v4();
    let message = Uuid::new_v4();
    let content = "[FICTIF] Une demande conservée.";
    let client = reqwest::Client::new();
    client
        .post(format!("{path}/messages"))
        .header("Idempotency-Key", key.to_string())
        .json(&json!({"content":content,"client_message_id":message}))
        .send()
        .await?;
    sqlx::query("update app.idempotency_records set status='processing',response_status=null,response_body=null,error_code=null,locked_until=now()+interval '1 minute' where idempotency_key=$1 and actor_id=$2")
        .bind(key.to_string()).bind(owner.actor_id).execute(&admin).await?;
    let view: Value = client.get(&path).send().await?.json().await?;
    ensure!(
        view["message_commands"][0]["status"] == "processing"
            && view["message_commands"][0]["can_retry"] == false
    );
    let retry = client
        .post(format!("{path}/messages"))
        .header("Idempotency-Key", key.to_string())
        .json(&json!({"content":content,"client_message_id":message}))
        .send()
        .await?;
    ensure!(retry.status().as_u16() == 409 && engine.messages.lock().unwrap().len() == 1);
    sqlx::query("update app.idempotency_records set locked_until=now()-interval '1 minute' where idempotency_key=$1 and actor_id=$2").bind(key.to_string()).bind(owner.actor_id).execute(&admin).await?;
    let view: Value = client.get(&path).send().await?.json().await?;
    ensure!(
        view["message_commands"][0]["status"] == "interrupted"
            && view["message_commands"][0]["can_retry"] == true
    );
    sqlx::query("update app.idempotency_records set created_at=now()-interval '2 days',expires_at=now()-interval '1 minute' where idempotency_key=$1 and actor_id=$2").bind(key.to_string()).bind(owner.actor_id).execute(&admin).await?;
    sqlx::query("insert into app.messages(workspace_id,project_id,session_id,role,content) select workspace_id,project_id,id,'user','[FICTIF] Message historique sans auteur' from app.sessions where public_id=$1").bind(session).execute(&admin).await?;
    let view: Value = client.get(&path).send().await?.json().await?;
    ensure!(
        view["message_commands"][0]["status"] == "expired"
            && view["message_commands"][0]["can_retry"] == false
    );
    let legacy = &view["messages"][1];
    ensure!(
        legacy["author_actor_id"].is_null()
            && legacy["author_name"].is_null()
            && legacy["is_own"] == false
    );
    ensure!(engine.messages.lock().unwrap().len() == 1);
    // Another owner must remain while the author is removed.
    sqlx::query(
        "update app.workspace_members set role='owner' where workspace_id=$1 and actor_id<>$2",
    )
    .bind(owner.workspace_internal_id)
    .bind(owner.actor_id)
    .execute(&admin)
    .await?;
    sqlx::query("update app.workspace_members set invitation_status='revoked' where workspace_id=$1 and actor_id=$2").bind(owner.workspace_internal_id).bind(owner.actor_id).execute(&admin).await?;
    ensure!(!client.get(&path).send().await?.status().is_success());
    server.abort();
    owner.pool.close().await;
    admin.close().await;
    Ok(())
}
