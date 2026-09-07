//! Personal provider connections. Only this module can read encrypted credentials.
#![allow(clippy::missing_errors_doc, clippy::too_many_lines)]
pub mod encryption;

use crate::{
    agent::{AgentEngine, DeterministicEngine, StructuredEngine},
    error::{AppError, AppResult},
    idempotency::{self, IdempotencyLease},
    integrations::providers::{ProviderModel, StructuredTransport, api_transport},
    provider_subscriptions::{SubscriptionRuntime, SubscriptionScope},
    service::AppState,
};
use chrono::{DateTime, Utc};
use encryption::CredentialCipher;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use std::sync::Arc;
use uuid::Uuid;

pub struct ProviderRuntime {
    pub cipher: Option<CredentialCipher>,
    pub subscriptions: Option<Arc<SubscriptionRuntime>>,
}

#[derive(Serialize)]
pub struct Provider {
    pub id: &'static str,
    pub name: &'static str,
    pub auth_method: &'static str,
    pub available: bool,
    pub unavailable_reason: Option<String>,
    pub help_url: &'static str,
}
#[derive(Serialize)]
pub struct StorageStatus {
    pub available: bool,
    pub message: &'static str,
}
#[derive(Serialize)]
pub struct Settings {
    pub providers: Vec<Provider>,
    pub connections: Vec<Connection>,
    pub selection: Selection,
    pub storage: StorageStatus,
}
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Connection {
    pub id: Uuid,
    pub provider: String,
    pub name: String,
    pub model: String,
    pub key_hint: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, sqlx::FromRow)]
pub struct Selection {
    pub mode: String,
    pub connection_id: Option<Uuid>,
}
impl Default for Selection {
    fn default() -> Self {
        Self {
            mode: "server_default".into(),
            connection_id: None,
        }
    }
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CreateConnection {
    pub id: Uuid,
    pub provider: String,
    pub name: String,
    pub model: String,
    #[serde(default, deserialize_with = "secret_input", skip_serializing)]
    pub api_key: Option<SecretString>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateConnection {
    pub name: String,
    pub model: String,
    #[serde(default, deserialize_with = "secret_input", skip_serializing)]
    pub api_key: Option<SecretString>,
}
#[derive(Serialize)]
pub struct ConnectionTest {
    pub ok: bool,
    pub models: Vec<ProviderModel>,
}

fn secret_input<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<SecretString>, D::Error> {
    Option::<String>::deserialize(deserializer).map(|value| value.map(SecretString::from))
}

pub fn request_commitment<T: Serialize>(
    state: &AppState,
    request: &T,
    secret: Option<&SecretString>,
) -> AppResult<Value> {
    // The idempotency table receives only a keyed one-way commitment, never
    // the credential or a reversible envelope. Serialization itself omits keys.
    Ok(
        json!({"request":request,"credential_commitment":secret.map(|value| cipher(state).map(|cipher|cipher.commitment(value))).transpose()?}),
    )
}

fn require_editor(state: &AppState) -> AppResult<()> {
    if matches!(state.workspace_role.as_str(), "owner" | "editor") {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}
fn workspace(state: &AppState) -> AppResult<i64> {
    state
        .workspace_internal_id
        .ok_or_else(|| AppError::Internal("workspace scope is missing".into()))
}
fn cipher(state: &AppState) -> AppResult<&CredentialCipher> {
    state
        .providers
        .as_ref()
        .and_then(|runtime| runtime.cipher.as_ref())
        .ok_or_else(encryption::storage_error)
}
fn subscription_provider(provider: &str) -> bool {
    matches!(provider, "claude_subscription" | "codex_subscription")
}
fn valid_provider(provider: &str) -> bool {
    matches!(
        provider,
        "openai" | "anthropic" | "kimi" | "deepseek" | "openrouter" | "claude_subscription"
    )
}

pub async fn settings(state: &AppState) -> AppResult<Settings> {
    require_editor(state)?;
    let available = state
        .providers
        .as_ref()
        .is_some_and(|runtime| runtime.cipher.is_some());
    let mut providers = [
        ("openai", "OpenAI", "https://platform.openai.com/api-keys"),
        (
            "anthropic",
            "Anthropic",
            "https://console.anthropic.com/settings/keys",
        ),
        (
            "kimi",
            "Kimi / Moonshot",
            "https://platform.moonshot.ai/console/api-keys",
        ),
        (
            "deepseek",
            "DeepSeek",
            "https://platform.deepseek.com/api_keys",
        ),
        (
            "openrouter",
            "OpenRouter",
            "https://openrouter.ai/settings/keys",
        ),
    ]
    .into_iter()
    .map(|(id, name, help_url)| Provider {
        id,
        name,
        help_url,
        auth_method: "api_key",
        available,
        unavailable_reason: (!available)
            .then(|| "Le stockage sécurisé n’est pas configuré sur le serveur.".into()),
    })
    .collect::<Vec<_>>();
    let capability = if let Some(runtime) = state
        .providers
        .as_ref()
        .and_then(|runtime| runtime.subscriptions.as_ref())
    {
        Some(runtime.capabilities().await)
    } else {
        None
    };
    providers.push(Provider{id:"claude_subscription",name:"Claude (abonnement)",auth_method:"subscription",available:capability.as_ref().is_some_and(|capability|capability.available),unavailable_reason:capability.and_then(|capability|capability.reason).or_else(||state.providers.as_ref().and_then(|runtime|runtime.subscriptions.as_ref()).is_none().then(||"Le client Claude officiel doit être activé par l’administrateur du serveur.".into())),help_url:"https://code.claude.com/docs/en/overview"});
    providers.push(Provider{id:"codex_subscription",name:"ChatGPT / Codex (abonnement)",auth_method:"subscription",available:false,unavailable_reason:Some("Ce mode n’offre pas encore les garanties d’isolation requises par AI Center. Utilisez une connexion API OpenAI.".into()),help_url:"https://developers.openai.com/codex/auth"});
    let mut tx = state.begin_request().await?;
    let connections=sqlx::query_as::<_,Connection>("select public_id as id,provider,name,model,key_hint,created_at,updated_at from app.provider_connections where workspace_id=$1 and actor_id=$2 order by created_at,public_id").bind(workspace(state)?).bind(state.actor_id).fetch_all(&mut *tx).await?;
    let selection = read_selection(state, &mut tx).await?;
    tx.commit().await?;
    Ok(Settings {
        providers,
        connections,
        selection,
        storage: StorageStatus {
            available,
            message: if available {
                "Les clés sont chiffrées côté serveur et ne sont jamais renvoyées à l’application."
            } else {
                "Le stockage sécurisé des clés doit être configuré sur le serveur."
            },
        },
    })
}

async fn read_selection(
    state: &AppState,
    tx: &mut Transaction<'_, Postgres>,
) -> AppResult<Selection> {
    Ok(sqlx::query_as::<_,Selection>("select mode,connection_id from app.provider_selections where workspace_id=$1 and actor_id=$2").bind(workspace(state)?).bind(state.actor_id).fetch_optional(&mut **tx).await?.unwrap_or_default())
}
async fn lock_actor(state: &AppState, tx: &mut Transaction<'_, Postgres>) -> AppResult<()> {
    // Serialize settings changes, deletions and engine resolution for one
    // actor/workspace even when no selection row exists yet.
    sqlx::query("select pg_advisory_xact_lock(hashtextextended($1, 21983))")
        .bind(format!("{}:{}", workspace(state)?, state.actor_id))
        .execute(&mut **tx)
        .await?;
    Ok(())
}
fn validated_text(value: &str, maximum: usize, label: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > maximum || value.chars().any(char::is_control) {
        return Err(AppError::Invalid(format!("{label} is missing or invalid")));
    }
    Ok(value.to_owned())
}
fn encrypted_key(
    state: &AppState,
    id: Uuid,
    provider: &str,
    key: Option<&SecretString>,
) -> AppResult<(Option<Vec<u8>>, Option<String>)> {
    if subscription_provider(provider) {
        if key.is_some() {
            return Err(AppError::Invalid(
                "Subscription connections do not accept API credentials".into(),
            ));
        }
        return Ok((None, None));
    }
    let key = key.ok_or_else(|| AppError::Invalid("An API key is required".into()))?;
    let value = key.expose_secret();
    if value.len() < 8
        || value.len() > 4096
        || value.chars().any(char::is_whitespace)
        || !value.is_ascii()
    {
        return Err(AppError::Invalid("The API key format is invalid".into()));
    }
    let envelope = cipher(state)?.encrypt(state.workspace_id, state.actor_id, id, provider, key)?;
    Ok((
        Some(envelope),
        Some(format!("••••{}", &value[value.len() - 4..])),
    ))
}

async fn complete<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    lease: Option<&IdempotencyLease>,
    value: &T,
) -> AppResult<()> {
    if let Some(lease) = lease {
        idempotency::complete(
            tx,
            lease,
            200,
            serde_json::to_value(value)
                .map_err(|_| AppError::Internal("provider response serialization failed".into()))?,
        )
        .await?;
    }
    Ok(())
}

pub async fn create(
    state: &AppState,
    input: CreateConnection,
    lease: Option<&IdempotencyLease>,
) -> AppResult<Connection> {
    require_editor(state)?;
    if !valid_provider(&input.provider) {
        return Err(AppError::Invalid("Unsupported provider".into()));
    }
    let name = validated_text(&input.name, 120, "Connection name")?;
    let model = validated_text(&input.model, 200, "Model")?;
    let (ciphertext, hint) =
        encrypted_key(state, input.id, &input.provider, input.api_key.as_ref())?;
    let mut tx = state.begin_request().await?;
    lock_actor(state, &mut tx).await?;
    let connection=sqlx::query_as::<_,Connection>("insert into app.provider_connections (public_id,workspace_id,actor_id,provider,name,model,key_ciphertext,key_hint) values ($1,$2,$3,$4,$5,$6,$7,$8) on conflict do nothing returning public_id as id,provider,name,model,key_hint,created_at,updated_at").bind(input.id).bind(workspace(state)?).bind(state.actor_id).bind(input.provider).bind(name).bind(model).bind(ciphertext).bind(hint).fetch_optional(&mut *tx).await?.ok_or_else(||AppError::Conflict("The connection already exists".into()))?;
    complete(&mut tx, lease, &connection).await?;
    tx.commit().await?;
    Ok(connection)
}
pub async fn update(
    state: &AppState,
    id: Uuid,
    input: UpdateConnection,
    lease: Option<&IdempotencyLease>,
) -> AppResult<Connection> {
    require_editor(state)?;
    let name = validated_text(&input.name, 120, "Connection name")?;
    let model = validated_text(&input.model, 200, "Model")?;
    let mut tx = state.begin_request().await?;
    lock_actor(state, &mut tx).await?;
    let provider:String=sqlx::query_scalar("select provider from app.provider_connections where public_id=$1 and workspace_id=$2 and actor_id=$3 for update").bind(id).bind(workspace(state)?).bind(state.actor_id).fetch_optional(&mut *tx).await?.ok_or(AppError::NotFound)?;
    let replacement = input.api_key.is_some();
    let (ciphertext, hint) = if replacement {
        encrypted_key(state, id, &provider, input.api_key.as_ref())?
    } else {
        (None, None)
    };
    let connection=sqlx::query_as::<_,Connection>("update app.provider_connections set name=$4, model=$5,key_ciphertext=case when $6 then $7 else key_ciphertext end,key_hint=case when $6 then $8 else key_hint end,updated_at=clock_timestamp() where public_id=$1 and workspace_id=$2 and actor_id=$3 returning public_id as id,provider,name,model,key_hint,created_at,updated_at").bind(id).bind(workspace(state)?).bind(state.actor_id).bind(name).bind(model).bind(replacement).bind(ciphertext).bind(hint).fetch_one(&mut *tx).await?;
    complete(&mut tx, lease, &connection).await?;
    tx.commit().await?;
    Ok(connection)
}
pub async fn delete(
    state: &AppState,
    id: Uuid,
    lease: Option<&IdempotencyLease>,
) -> AppResult<Value> {
    require_editor(state)?;
    let mut tx = state.begin_request().await?;
    lock_actor(state, &mut tx).await?;
    let connection:Option<String>=sqlx::query_scalar("select provider from app.provider_connections where public_id=$1 and workspace_id=$2 and actor_id=$3 for update").bind(id).bind(workspace(state)?).bind(state.actor_id).fetch_optional(&mut *tx).await?;
    let provider = connection.ok_or(AppError::NotFound)?;
    // Sessions must be revoked before deleting the metadata that owns them.
    if subscription_provider(&provider) {
        let (runtime, scope) = subscription_runtime_for(state, id)?;
        runtime.forget(scope).await?;
    }
    sqlx::query("update app.provider_selections set mode='deterministic',connection_id=null,updated_at=clock_timestamp() where workspace_id=$1 and actor_id=$2 and connection_id=$3").bind(workspace(state)?).bind(state.actor_id).bind(id).execute(&mut *tx).await?;
    sqlx::query("delete from app.provider_connections where public_id=$1 and workspace_id=$2 and actor_id=$3").bind(id).bind(workspace(state)?).bind(state.actor_id).execute(&mut *tx).await?;
    let result = json!({"deleted":true});
    complete(&mut tx, lease, &result).await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn select(
    state: &AppState,
    input: Selection,
    lease: Option<&IdempotencyLease>,
) -> AppResult<Selection> {
    require_editor(state)?;
    if !matches!(
        input.mode.as_str(),
        "server_default" | "deterministic" | "connection"
    ) || (input.mode == "connection") != input.connection_id.is_some()
    {
        return Err(AppError::Invalid("The engine selection is invalid".into()));
    }
    let mut tx = state.begin_request().await?;
    lock_actor(state, &mut tx).await?;
    if let Some(id) = input.connection_id
        && !sqlx::query_scalar::<_,bool>("select exists(select 1 from app.provider_connections where public_id=$1 and workspace_id=$2 and actor_id=$3)").bind(id).bind(workspace(state)?).bind(state.actor_id).fetch_one(&mut *tx).await? { return Err(AppError::NotFound); }
    sqlx::query("insert into app.provider_selections(workspace_id,actor_id,mode,connection_id) values ($1,$2,$3,$4) on conflict (workspace_id,actor_id) do update set mode=excluded.mode,connection_id=excluded.connection_id,updated_at=clock_timestamp()").bind(workspace(state)?).bind(state.actor_id).bind(&input.mode).bind(input.connection_id).execute(&mut *tx).await?;
    complete(&mut tx, lease, &input).await?;
    tx.commit().await?;
    Ok(input)
}

async fn read_transport(
    state: &AppState,
    id: Uuid,
    tx: &mut Transaction<'_, Postgres>,
) -> AppResult<(Arc<dyn StructuredTransport>, String)> {
    let (provider,model,ciphertext):(String,String,Option<Vec<u8>>)=sqlx::query_as("select provider,model,key_ciphertext from app.provider_connections where public_id=$1 and workspace_id=$2 and actor_id=$3").bind(id).bind(workspace(state)?).bind(state.actor_id).fetch_optional(&mut **tx).await?.ok_or(AppError::NotFound)?;
    let transport = if subscription_provider(&provider) {
        let (runtime, scope) = subscription_runtime_for(state, id)?;
        runtime.transport(scope)
    } else {
        api_transport(
            &provider,
            cipher(state)?.decrypt(
                state.workspace_id,
                state.actor_id,
                id,
                &provider,
                &ciphertext.ok_or_else(encryption::storage_error)?,
            )?,
        )?
    };
    Ok((transport, model))
}
pub async fn resolve_engine(state: &AppState) -> AppResult<Arc<dyn AgentEngine>> {
    require_editor(state)?;
    // None exists only in deterministic test fixtures / legacy embedded callers.
    if state.providers.is_none() {
        return Ok(Arc::clone(&state.engine));
    }
    let mut tx = state.begin_request().await?;
    lock_actor(state, &mut tx).await?;
    let selection = read_selection(state, &mut tx).await?;
    let engine: Arc<dyn AgentEngine> = match selection.mode.as_str() {
        "server_default" => Arc::clone(&state.engine),
        "deterministic" => Arc::new(DeterministicEngine),
        "connection" => {
            let (transport, model) = read_transport(
                state,
                selection
                    .connection_id
                    .ok_or_else(encryption::storage_error)?,
                &mut tx,
            )
            .await?;
            Arc::new(StructuredEngine::with_transport(transport, model))
        }
        _ => {
            return Err(AppError::Agent(
                "The selected AI connection is invalid".into(),
            ));
        }
    };
    tx.commit().await?;
    Ok(engine)
}
pub async fn test(state: &AppState, id: Uuid) -> AppResult<ConnectionTest> {
    require_editor(state)?;
    let mut tx = state.begin_request().await?;
    let (transport, _) = read_transport(state, id, &mut tx).await?;
    tx.commit().await?;
    let models = transport.list_models().await?;
    Ok(ConnectionTest { ok: true, models })
}

fn subscription_runtime_for(
    state: &AppState,
    id: Uuid,
) -> AppResult<(Arc<SubscriptionRuntime>, SubscriptionScope)> {
    let runtime = state
        .providers
        .as_ref()
        .and_then(|runtime| runtime.subscriptions.clone())
        .ok_or_else(|| {
            AppError::Agent("Le client officiel d’abonnement n’est pas configuré.".into())
        })?;
    Ok((
        runtime,
        SubscriptionScope {
            workspace_id: state.workspace_id,
            actor_id: state.actor_id,
            connection_id: id,
        },
    ))
}
pub async fn subscription(
    state: &AppState,
    id: Uuid,
) -> AppResult<(Arc<SubscriptionRuntime>, SubscriptionScope)> {
    require_editor(state)?;
    let mut tx = state.begin_request().await?;
    let provider:Option<String>=sqlx::query_scalar("select provider from app.provider_connections where public_id=$1 and workspace_id=$2 and actor_id=$3").bind(id).bind(workspace(state)?).bind(state.actor_id).fetch_optional(&mut *tx).await?;
    tx.commit().await?;
    if provider.as_deref() != Some("claude_subscription") {
        return Err(AppError::NotFound);
    }
    subscription_runtime_for(state, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn submitted_secrets_are_never_serialized() {
        let raw = Uuid::new_v4().to_string();
        let input:CreateConnection=serde_json::from_value(json!({"id":Uuid::new_v4(),"provider":"openai","name":"test","model":"test-model","api_key":raw})).unwrap();
        let output = serde_json::to_string(&input).unwrap();
        assert!(!output.contains(&raw));
        assert!(!output.contains("api_key"));
    }
}
