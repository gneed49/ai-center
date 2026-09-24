//! Company-owned work tools and explicit durable external publications.
#![allow(clippy::missing_errors_doc, clippy::too_many_lines)]
pub mod client;
pub mod code;
mod code_privacy;
pub mod models;
mod publications;
pub mod reliability;
pub mod ticket_models;
mod ticket_projection;
pub mod tickets;
pub mod worker;
pub use models::*;
pub use publications::{cancel, detail, list, publish, reconcile, refresh};

/// Local contract-test entry point; injected clients only accept loopback URLs.
#[cfg(debug_assertions)]
pub async fn observe_with_client(
    state: &AppState,
    id: Uuid,
    external_id: Option<String>,
    client: &client::ToolClient,
) -> AppResult<Publication> {
    publications::observe(state, id, external_id, None, client).await
}

use crate::{
    artifacts::{complete, editor},
    error::{AppError, AppResult},
    idempotency::IdempotencyLease,
    providers::encryption::CredentialCipher,
    service::AppState,
};
use secrecy::ExposeSecret;
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

const CONNECTION_SELECT: &str = "select public_id,provider,name,enabled,revision,created_at,updated_at from app.work_tool_connections";
pub(crate) fn cipher(state: &AppState) -> AppResult<&CredentialCipher> {
    state
        .providers
        .as_ref()
        .and_then(|runtime| runtime.cipher.as_ref())
        .ok_or_else(|| {
            AppError::Invalid("Secure connection storage is unavailable on this server".into())
        })
}
fn owner(state: &AppState) -> AppResult<()> {
    if state.workspace_role == "owner" {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}
pub(crate) async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    state: &AppState,
    action: &str,
    object: Uuid,
    details: Value,
) -> AppResult<()> {
    sqlx::query("insert into app.audit_events(workspace_id,actor_id,action,object_kind,object_public_id,after_state) values(app.current_workspace_id(),$1,$2,'work_tool',$3,$4)")
        .bind(state.actor_id).bind(action).bind(object).bind(details).execute(&mut **tx).await?;
    Ok(())
}
pub async fn settings(state: &AppState) -> AppResult<Settings> {
    let mut tx = state.begin_request().await?;
    let connections=sqlx::query_as(&format!("{CONNECTION_SELECT} where workspace_id=app.current_workspace_id() order by provider,name,public_id"))
        .fetch_all(&mut *tx).await?;
    let usage = reliability::usage(&mut tx).await?;
    tx.commit().await?;
    Ok(Settings {
        limits: reliability::limits()?,
        usage,
        storage_available: cipher(state).is_ok(),
        connections,
        capabilities: ["notion", "linear", "github"]
            .into_iter()
            .map(|provider| Capability {
                provider,
                create: true,
                read: true,
                reconcile: true,
                update: false,
            })
            .collect(),
    })
}
fn validate_connection(input: &SaveConnection) -> AppResult<()> {
    if input.id.is_nil()
        || !matches!(input.provider.as_str(), "notion" | "linear" | "github")
        || input.expected_revision < 0
        || input.name.trim().is_empty()
        || input.name.trim().chars().count() > 120
    {
        return Err(AppError::Invalid("Invalid tool connection settings".into()));
    }
    if let Some(secret) = &input.api_key {
        let text = secret.expose_secret();
        if !(8..=4096).contains(&text.len()) || !text.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err(AppError::Invalid("Invalid connection credential".into()));
        }
    } else if input.expected_revision == 0 {
        return Err(AppError::Invalid(
            "A credential is required for a new connection".into(),
        ));
    }
    Ok(())
}
pub async fn save_connection(
    state: &AppState,
    input: SaveConnection,
    lease: Option<&IdempotencyLease>,
) -> AppResult<Connection> {
    owner(state)?;
    validate_connection(&input)?;
    let mut tx = state.begin_request().await?;
    sqlx::query("select pg_advisory_xact_lock(hashtextextended(app.current_workspace_id()::text||':work-tool:'||$1,0))")
        .bind(input.id.to_string()).execute(&mut *tx).await?;
    let existing:Option<(i32,String)>=sqlx::query_as("select revision,provider from app.work_tool_connections where public_id=$1 and workspace_id=app.current_workspace_id() for update")
        .bind(input.id).fetch_optional(&mut *tx).await?;
    if existing.as_ref().map_or(0, |row| row.0) != input.expected_revision
        || existing.as_ref().is_some_and(|row| row.1 != input.provider)
    {
        return Err(AppError::Conflict(
            "Connection changed; reload before saving".into(),
        ));
    }
    let encrypted = input
        .api_key
        .as_ref()
        .map(|secret| {
            cipher(state)?.encrypt(
                state.workspace_id,
                state.actor_id,
                input.id,
                &input.provider,
                secret,
            )
        })
        .transpose()?;
    if input.expected_revision == 0 {
        sqlx::query("insert into app.work_tool_connections(public_id,workspace_id,provider,name,encrypted_credential,credential_actor_id) values($1,app.current_workspace_id(),$2,$3,$4,$5)")
            .bind(input.id).bind(&input.provider).bind(input.name.trim()).bind(encrypted).bind(state.actor_id).execute(&mut *tx).await?;
    } else {
        sqlx::query("update app.work_tool_connections set name=$2,encrypted_credential=coalesce($3,encrypted_credential),credential_actor_id=case when $3::bytea is null then credential_actor_id else $4 end,enabled=true,revision=revision+1,updated_at=now() where public_id=$1 and workspace_id=app.current_workspace_id()")
            .bind(input.id).bind(input.name.trim()).bind(encrypted).bind(state.actor_id).execute(&mut *tx).await?;
    }
    let result = sqlx::query_as(&format!(
        "{CONNECTION_SELECT} where public_id=$1 and workspace_id=app.current_workspace_id()"
    ))
    .bind(input.id)
    .fetch_one(&mut *tx)
    .await?;
    audit(
        &mut tx,
        state,
        "work_tool.connection.saved",
        input.id,
        json!({"provider":input.provider,"credential_changed":input.api_key.is_some()}),
    )
    .await?;
    complete(&mut tx, lease, &result).await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn disable_connection(
    state: &AppState,
    id: Uuid,
    input: DisableConnection,
    lease: Option<&IdempotencyLease>,
) -> AppResult<Connection> {
    owner(state)?;
    let mut tx = state.begin_request().await?;
    let result:Option<Connection>=sqlx::query_as("update app.work_tool_connections set enabled=false,revision=revision+1,updated_at=now() where public_id=$1 and workspace_id=app.current_workspace_id() and revision=$2 returning public_id,provider,name,enabled,revision,created_at,updated_at")
        .bind(id).bind(input.expected_revision).fetch_optional(&mut *tx).await?;
    let result =
        result.ok_or_else(|| AppError::Conflict("Connection changed or is unavailable".into()))?;
    audit(
        &mut tx,
        state,
        "work_tool.connection.disabled",
        id,
        json!({}),
    )
    .await?;
    complete(&mut tx, lease, &result).await?;
    tx.commit().await?;
    Ok(result)
}
pub(crate) async fn credential(state: &AppState, id: Uuid) -> AppResult<Credential> {
    editor(state)?;
    let mut tx = state.begin_request().await?;
    let row:Option<(String,i32,Uuid,Vec<u8>)>=sqlx::query_as("select provider,revision,credential_actor_id,encrypted_credential from app.work_tool_connections where public_id=$1 and workspace_id=app.current_workspace_id() and enabled and app.has_workspace_role(workspace_id,array['owner','editor']::text[])")
        .bind(id).fetch_optional(&mut *tx).await?;
    tx.commit().await?;
    let (provider, revision, actor, envelope) = row.ok_or(AppError::NotFound)?;
    let secret = cipher(state)?.decrypt(state.workspace_id, actor, id, &provider, &envelope)?;
    Ok(Credential {
        provider,
        revision,
        secret,
    })
}
pub async fn test_connection(state: &AppState, id: Uuid) -> AppResult<ConnectionTest> {
    owner(state)?;
    let credential = credential(state, id).await?;
    reliability::admit_read(state, id).await?;
    let result = client::ToolClient::official()?
        .test(&credential.provider, &credential.secret)
        .await;
    let result = match result {
        Ok(()) => ConnectionTest {
            ok: true,
            code: "credential_verified".into(),
        },
        Err(error) => ConnectionTest {
            ok: false,
            code: error.code.into(),
        },
    };
    let mut tx = state.begin_request().await?;
    audit(
        &mut tx,
        state,
        "work_tool.connection.tested",
        id,
        json!({"ok":result.ok,"code":result.code}),
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credentials_are_never_serialized_and_validation_is_bounded() {
        let input:SaveConnection=serde_json::from_value(json!({"id":Uuid::new_v4(),"provider":"notion","name":"[FICTIF] Notion","expected_revision":0,"api_key":"synthetic-secret-only"})).expect("input");
        assert!(validate_connection(&input).is_ok());
        let output = serde_json::to_string(&input).expect("json");
        assert!(!output.contains("synthetic-secret-only"));
        assert!(!output.contains("api_key"));
        let invalid:SaveConnection=serde_json::from_value(json!({"id":Uuid::new_v4(),"provider":"github","name":"Test","expected_revision":0,"api_key":"secret\nheader"})).expect("input");
        assert!(validate_connection(&invalid).is_err());
        assert!(client::normalize_external_id("github", "1/../../user").is_none());
        assert!(client::normalize_external_id("notion", "https://evil.example/").is_none());
    }
}
