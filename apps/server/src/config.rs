use std::{env, net::SocketAddr, path::PathBuf};

use anyhow::{Context, Result};
use axum::http::HeaderValue;
use secrecy::SecretString;
use uuid::Uuid;

#[derive(Clone)]
pub struct Config {
    pub bind: SocketAddr,
    pub database_url: SecretString,
    pub openai_api_key: Option<SecretString>,
    pub openai_model: String,
    pub agent_mode: AgentMode,
    pub auth_mode: AuthMode,
    pub supabase_url: Option<String>,
    pub cors_origins: Vec<HeaderValue>,
    pub github_app_id: Option<String>,
    pub github_app_installation_id: Option<u64>,
    pub github_app_private_key_path: Option<PathBuf>,
    pub workspace_id: Uuid,
    pub actor_id: Uuid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentMode {
    OpenAi,
    Deterministic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthMode {
    Local,
    Supabase,
}

impl Config {
    /// Loads the deployable server configuration from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error when an address or UUID is malformed, the agent mode is
    /// unsupported, or `OpenAI` mode is selected without a server-side key.
    #[allow(clippy::too_many_lines)]
    pub fn from_env() -> Result<Self> {
        // Local overrides are deliberately untracked; production uses real env variables.
        let _ = dotenvy::from_filename(".env.local");
        let _ = dotenvy::dotenv();

        let bind: SocketAddr = env::var("AI_CENTER_BIND")
            .unwrap_or_else(|_| "127.0.0.1:4317".into())
            .parse()
            .context("AI_CENTER_BIND must be a socket address")?;
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:postgres@127.0.0.1:54322/postgres".into());
        let openai_api_key = env::var("OPENAI_API_KEY").ok().map(SecretString::from);
        let agent_mode = match env::var("AI_CENTER_AGENT_MODE")
            .unwrap_or_else(|_| "deterministic".into())
            .as_str()
        {
            "deterministic" => AgentMode::Deterministic,
            "openai" => AgentMode::OpenAi,
            value => anyhow::bail!("unsupported AI_CENTER_AGENT_MODE: {value}"),
        };

        let openai_model = env::var("OPENAI_MODEL").unwrap_or_default();
        if agent_mode == AgentMode::OpenAi
            && (openai_api_key.is_none() || openai_model.trim().is_empty())
        {
            anyhow::bail!(
                "OPENAI_API_KEY and an explicitly calibrated OPENAI_MODEL are required when AI_CENTER_AGENT_MODE=openai"
            );
        }

        let auth_mode = match env::var("AI_CENTER_AUTH_MODE")
            .unwrap_or_else(|_| "local".into())
            .as_str()
        {
            "local" => AuthMode::Local,
            "supabase" => AuthMode::Supabase,
            value => anyhow::bail!("unsupported AI_CENTER_AUTH_MODE: {value}"),
        };
        let supabase_url = env::var("SUPABASE_URL")
            .ok()
            .map(|value| value.trim_end_matches('/').to_owned());
        if auth_mode == AuthMode::Supabase && supabase_url.is_none() {
            anyhow::bail!("SUPABASE_URL is required when AI_CENTER_AUTH_MODE=supabase");
        }
        if auth_mode == AuthMode::Local && !bind.ip().is_loopback() {
            anyhow::bail!("AI_CENTER_AUTH_MODE=local requires a loopback AI_CENTER_BIND");
        }

        let cors_origins = env::var("AI_CENTER_CORS_ORIGINS")
            .unwrap_or_else(|_| {
                [
                    "http://127.0.0.1:5173",
                    "http://localhost:5173",
                    "http://tauri.localhost",
                    "https://tauri.localhost",
                    "tauri://localhost",
                ]
                .join(",")
            })
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| {
                if value == "*" {
                    anyhow::bail!("AI_CENTER_CORS_ORIGINS cannot contain a wildcard");
                }
                HeaderValue::from_str(value)
                    .with_context(|| format!("invalid AI_CENTER_CORS_ORIGINS value: {value}"))
            })
            .collect::<Result<Vec<_>>>()?;
        if cors_origins.is_empty() {
            anyhow::bail!("AI_CENTER_CORS_ORIGINS must contain at least one explicit origin");
        }

        let github_app_id = env::var("GITHUB_APP_ID").ok();
        let github_app_installation_id = env::var("GITHUB_APP_INSTALLATION_ID")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()
            .context("GITHUB_APP_INSTALLATION_ID must be a positive integer")?;
        let github_app_private_key_path = env::var("GITHUB_APP_PRIVATE_KEY_PATH")
            .ok()
            .map(PathBuf::from);
        let github_values = [
            github_app_id.is_some(),
            github_app_installation_id.is_some(),
            github_app_private_key_path.is_some(),
        ];
        if github_values.iter().any(|present| *present)
            && !github_values.iter().all(|present| *present)
        {
            anyhow::bail!(
                "GITHUB_APP_ID, GITHUB_APP_INSTALLATION_ID, and GITHUB_APP_PRIVATE_KEY_PATH must be configured together"
            );
        }

        Ok(Self {
            bind,
            database_url: SecretString::from(database_url),
            openai_api_key,
            openai_model,
            agent_mode,
            auth_mode,
            supabase_url,
            cors_origins,
            github_app_id,
            github_app_installation_id,
            github_app_private_key_path,
            workspace_id: parse_uuid(
                "AI_CENTER_WORKSPACE_ID",
                "10000000-0000-0000-0000-000000000001",
            )?,
            actor_id: parse_uuid("AI_CENTER_ACTOR_ID", "00000000-0000-0000-0000-000000000001")?,
        })
    }
}

fn parse_uuid(name: &str, default: &str) -> Result<Uuid> {
    env::var(name)
        .unwrap_or_else(|_| default.into())
        .parse()
        .with_context(|| format!("{name} must be a UUID"))
}
