use std::{env, net::SocketAddr};

use anyhow::{Context, Result};
use secrecy::SecretString;
use uuid::Uuid;

#[derive(Clone)]
pub struct Config {
    pub bind: SocketAddr,
    pub database_url: SecretString,
    pub openai_api_key: Option<SecretString>,
    pub openai_model: String,
    pub agent_mode: AgentMode,
    pub workspace_id: Uuid,
    pub actor_id: Uuid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentMode {
    OpenAi,
    Deterministic,
}

impl Config {
    /// Loads the deployable server configuration from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error when an address or UUID is malformed, the agent mode is
    /// unsupported, or `OpenAI` mode is selected without a server-side key.
    pub fn from_env() -> Result<Self> {
        // Local overrides are deliberately untracked; production uses real env variables.
        let _ = dotenvy::from_filename(".env.local");
        let _ = dotenvy::dotenv();

        let bind = env::var("AI_CENTER_BIND")
            .unwrap_or_else(|_| "127.0.0.1:4317".into())
            .parse()
            .context("AI_CENTER_BIND must be a socket address")?;
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:postgres@127.0.0.1:54322/postgres".into());
        let openai_api_key = env::var("OPENAI_API_KEY").ok().map(SecretString::from);
        let agent_mode = match env::var("AI_CENTER_AGENT_MODE")
            .unwrap_or_else(|_| "openai".into())
            .as_str()
        {
            "deterministic" => AgentMode::Deterministic,
            "openai" => AgentMode::OpenAi,
            value => anyhow::bail!("unsupported AI_CENTER_AGENT_MODE: {value}"),
        };

        if agent_mode == AgentMode::OpenAi && openai_api_key.is_none() {
            anyhow::bail!("OPENAI_API_KEY is required when AI_CENTER_AGENT_MODE=openai");
        }

        Ok(Self {
            bind,
            database_url: SecretString::from(database_url),
            openai_api_key,
            openai_model: env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-5.6-luna".into()),
            agent_mode,
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
