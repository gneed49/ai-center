use super::super::models::{
    ExistingReadErrorCode as Code, ExistingReadFailure as Failure, ToolProvider,
};
use super::ExistingToolReader;
use chrono::Utc;
use reqwest::{Method, header};
use secrecy::{ExposeSecret, SecretString};
use serde_json::Value;

const MAX_RESPONSE_BYTES: usize = 262_144;
impl ExistingToolReader {
    pub(super) async fn request(
        &self,
        provider: ToolProvider,
        secret: &SecretString,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, Failure> {
        let origin = match provider {
            ToolProvider::Notion => "https://api.notion.com",
            ToolProvider::Linear => "https://api.linear.app",
        };
        let endpoint = self.loopback.as_ref().map_or_else(
            || format!("{origin}{path}"),
            |base| format!("{base}/{}{path}", provider.as_str()),
        );
        let auth = match provider {
            ToolProvider::Linear => secret.expose_secret().to_owned(),
            ToolProvider::Notion => format!("Bearer {}", secret.expose_secret()),
        };
        let mut auth = header::HeaderValue::from_str(&auth)
            .map_err(|_| Failure::new(Code::RemoteAuthentication))?;
        auth.set_sensitive(true);
        let method = if body.is_some() {
            Method::POST
        } else {
            Method::GET
        };
        let mut request = self
            .client
            .request(method, endpoint)
            .header(header::AUTHORIZATION, auth)
            .header(header::ACCEPT, "application/json");
        if provider == ToolProvider::Notion {
            request = request.header("Notion-Version", "2026-03-11");
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        let mut response = request.send().await.map_err(|e| {
            Failure::new(if e.is_timeout() {
                Code::RemoteTimeout
            } else {
                Code::RemoteTransport
            })
        })?;
        let retry_after = response
            .headers()
            .get(header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(retry_seconds);
        let status = response.status();
        if !status.is_success() {
            let code = match status.as_u16() {
                401 => Code::RemoteAuthentication,
                403 => Code::RemotePermission,
                404 => Code::RemoteNotFound,
                408 | 504 => Code::RemoteTimeout,
                429 => Code::RemoteRateLimit,
                300..=399 => Code::RemoteRedirectBlocked,
                500..=599 => Code::RemoteUnavailable,
                _ => Code::RemoteResponseInvalid,
            };
            let mut failure = Failure::new(code);
            if code == Code::RemoteRateLimit {
                failure.retry_after_seconds = Some(retry_after.unwrap_or(60));
            }
            return Err(failure);
        }
        if response
            .content_length()
            .is_some_and(|len| len > MAX_RESPONSE_BYTES as u64)
        {
            return Err(Failure::new(Code::RemoteResponseTooLarge));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|e| {
            Failure::new(if e.is_timeout() {
                Code::RemoteTimeout
            } else {
                Code::RemoteTransport
            })
        })? {
            if bytes.len() + chunk.len() > MAX_RESPONSE_BYTES {
                return Err(Failure::new(Code::RemoteResponseTooLarge));
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|_| Failure::new(Code::RemoteResponseInvalid))?;
        if provider == ToolProvider::Linear
            && let Some(errors) = value.get("errors")
        {
            let errors = errors
                .as_array()
                .ok_or_else(|| Failure::new(Code::RemoteResponseInvalid))?;
            if !errors.is_empty() {
                let rate_limited = errors.iter().any(|e| {
                    e.pointer("/extensions/code").and_then(Value::as_str) == Some("RATELIMITED")
                });
                let mut failure = Failure::new(if rate_limited {
                    Code::RemoteRateLimit
                } else {
                    Code::RemoteResponseInvalid
                });
                if rate_limited {
                    failure.retry_after_seconds = Some(retry_after.unwrap_or(60));
                }
                return Err(failure);
            }
        }
        Ok(value)
    }
}
fn retry_seconds(value: &str) -> Option<u32> {
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(u32::try_from(seconds.clamp(1, 86_400)).unwrap_or(86_400));
    }
    // HTTP-date is optional; malformed values use the conservative default.
    let date = chrono::DateTime::parse_from_rfc2822(value).ok()?;
    let seconds = (date.with_timezone(&Utc) - Utc::now())
        .num_seconds()
        .clamp(1, 86_400);
    u32::try_from(seconds).ok()
}
