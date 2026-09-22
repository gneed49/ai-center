//! Remote `PostgreSQL` connections must authenticate the server, not just encrypt
//! traffic. Local disposable stacks retain their explicit loopback exception.
use anyhow::{Result, bail};
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use std::{net::IpAddr, str::FromStr};

pub(crate) fn options(raw: &str) -> Result<PgConnectOptions> {
    let url = url::Url::parse(raw).map_err(|_| anyhow::anyhow!("Invalid DATABASE_URL"))?;
    if !matches!(url.scheme(), "postgres" | "postgresql") || url.host_str().is_none() {
        bail!("DATABASE_URL must explicitly name its PostgreSQL host");
    }
    let options = PgConnectOptions::from_str(raw)
        .map_err(|_| anyhow::anyhow!("Invalid PostgreSQL connection options"))?;
    let host = options
        .get_host()
        .trim_start_matches('[')
        .trim_end_matches(']');
    let loopback = host == "localhost"
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback());
    if !loopback && !matches!(options.get_ssl_mode(), PgSslMode::VerifyFull) {
        bail!("Remote DATABASE_URL requires sslmode=verify-full and a trusted server certificate");
    }
    Ok(options)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "requires scripts/postgres-tls-smoke.py on the guarded disposable stack"]
    async fn real_postgres_tls_verifies_certificate_and_hostname() -> Result<()> {
        use sqlx::Connection;
        let raw = std::env::var("AI_CENTER_RUNTIME_DATABASE_URL")?;
        let root = std::env::var("AI_CENTER_TLS_TEST_ROOT")?;
        let base = options(&raw)?;
        anyhow::ensure!(
            base.get_host() == "127.0.0.1"
                && base.get_port() == 55322
                && base.get_username() == "ai_center_runtime",
            "Exact isolated runtime target required"
        );
        let trusted = base
            .clone()
            .host("localhost")
            .ssl_mode(PgSslMode::VerifyFull)
            .ssl_root_cert(&root);
        let mut connection = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            sqlx::PgConnection::connect_with(&trusted),
        )
        .await??;
        let (encrypted, protocol): (bool, Option<String>) =
            sqlx::query_as("select ssl,version from pg_stat_ssl where pid=pg_backend_pid()")
                .fetch_one(&mut connection)
                .await?;
        anyhow::ensure!(
            encrypted && protocol.is_some_and(|value| value.starts_with("TLSv1.")),
            "Connection must actually use TLS"
        );
        connection.close().await?;
        let untrusted = base.host("localhost").ssl_mode(PgSslMode::VerifyFull);
        for options in [untrusted, trusted.host("127.0.0.1")] {
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                sqlx::PgConnection::connect_with(&options),
            )
            .await?;
            anyhow::ensure!(
                result.is_err(),
                "Untrusted CA or mismatched hostname must fail"
            );
        }
        Ok(())
    }
    #[test]
    fn remote_database_requires_hostname_and_certificate_verification() {
        for suffix in [
            "",
            "?sslmode=disable",
            "?sslmode=prefer",
            "?sslmode=require",
            "?sslmode=verify-ca",
        ] {
            assert!(
                options(&format!(
                    "postgresql://reader:synthetic@db.example.invalid/app{suffix}"
                ))
                .is_err()
            );
        }
        assert!(
            options("postgresql://reader:synthetic@db.example.invalid/app?sslmode=verify-full")
                .is_ok()
        );
        // SQLx's effective host, including a query override, determines transport policy.
        assert!(
            options("postgresql://reader:synthetic@127.0.0.1/app?host=db.example.invalid").is_err()
        );
    }
    #[test]
    fn loopback_fixtures_remain_usable_and_parse_errors_never_echo_credentials() {
        for host in ["127.0.0.1", "localhost", "[::1]"] {
            assert!(
                options(&format!(
                    "postgresql://reader:synthetic@{host}/app?sslmode=disable"
                ))
                .is_ok()
            );
        }
        for input in [
            "postgresql://reader:private-value@host.invalid:bad/app",
            "https://reader:private-value@host.invalid/app",
            "postgresql:///app",
        ] {
            let error = options(input).unwrap_err().to_string();
            assert!(!error.contains("private-value"));
        }
    }
}
