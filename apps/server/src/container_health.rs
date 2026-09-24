//! A bounded, loopback-only readiness probe for shell-free OCI images.
use anyhow::{Result, bail};
use std::{net::SocketAddr, time::Duration};

/// Checks the API's real database health through its local HTTP endpoint.
///
/// # Errors
/// Refuses non-loopback targets, timeouts, redirects and invalid health responses.
pub async fn check(address: SocketAddr) -> Result<()> {
    if !address.ip().is_loopback() {
        bail!("Health probe requires loopback");
    }
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(3))
        .build()?;
    let mut response = client
        .get(format!("http://{address}/api/health"))
        .send()
        .await?;
    if response.status() != reqwest::StatusCode::OK {
        bail!("API is not ready");
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if body.len() + chunk.len() > 4096 {
            bail!("Invalid health response size");
        }
        body.extend_from_slice(&chunk);
    }
    let body: serde_json::Value = serde_json::from_slice(&body)?;
    if body["status"] != "ok"
        || body["database"] != "connected"
        || body["service"] != "ai-center-server"
    {
        bail!("API database is not ready");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    async fn response(status: &str, body: &str) -> bool {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let wire = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0; 1024];
            let _ = stream.read(&mut buf).await;
            stream.write_all(wire.as_bytes()).await.unwrap();
        });
        let ok = check(address).await.is_ok();
        server.await.unwrap();
        ok
    }
    #[tokio::test]
    async fn readiness_needs_real_health_identity_database_and_success_status() {
        let good = r#"{"status":"ok","database":"connected","service":"ai-center-server"}"#;
        assert!(response("200 OK", good).await);
        assert!(!response("503 Unavailable", good).await);
        assert!(!response("302 Found", good).await);
        assert!(!response("200 OK", r#"{"status":"ok"}"#).await);
        assert!(!response("200 OK", &"x".repeat(4097)).await);
        assert!(check("192.0.2.1:4317".parse().unwrap()).await.is_err());
    }
}
