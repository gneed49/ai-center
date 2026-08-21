#![forbid(unsafe_code)]

use std::sync::Arc;

use anyhow::{Context, Result};
use secrecy::ExposeSecret;
use sqlx::postgres::PgPoolOptions;

use agent::{AgentEngine, DeterministicEngine, OpenAiEngine};
use config::{AgentMode, Config};
use service::AppState;

pub mod agent;
pub mod config;
pub mod error;
pub mod models;
pub mod routes;
pub mod service;

/// Connects the application services and returns the configured HTTP router.
///
/// # Errors
///
/// Returns an error when `PostgreSQL` cannot be reached or the selected agent
/// engine is missing required server-side configuration.
pub async fn build(config: Config) -> Result<(std::net::SocketAddr, axum::Router)> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(config.database_url.expose_secret())
        .await
        .context("failed to connect to PostgreSQL")?;
    let (engine, agent_mode): (Arc<dyn AgentEngine>, &'static str) = match config.agent_mode {
        AgentMode::Deterministic => (Arc::new(DeterministicEngine), "deterministic"),
        AgentMode::OpenAi => (
            Arc::new(OpenAiEngine::new(
                config.openai_api_key.context("OPENAI_API_KEY is missing")?,
                config.openai_model,
            )),
            "openai",
        ),
    };
    let bind = config.bind;
    let state = Arc::new(AppState {
        pool,
        engine,
        workspace_id: config.workspace_id,
        actor_id: config.actor_id,
        agent_mode,
    });
    Ok((bind, routes::router(state)))
}
