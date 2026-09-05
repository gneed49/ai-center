use ai_center_server::{build, config::Config};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ai_center_server=info,tower_http=info".into()),
        )
        .init();
    let (bind, app, steward_supervisor) = build(Config::from_env()?).await?;
    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(%bind, "AI Center server listening");
    let server_result = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await;
    if let Some(supervisor) = steward_supervisor {
        supervisor.shutdown().await;
    }
    server_result?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install terminate handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { () = ctrl_c => {}, () = terminate => {} }
}
