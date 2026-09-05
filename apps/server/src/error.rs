use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use thiserror::Error;

/// Stable failure classes for an external AI provider call.
///
/// This classification is deliberately independent from provider error text:
/// it drives durable retry decisions, HTTP status mapping, and `model_runs`
/// observability without parsing strings returned by a remote service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderErrorClass {
    Timeout,
    Connection,
    RateLimit,
    Quota,
    Server,
    Authentication,
    Permission,
    NotFound,
    Request,
    ResponseContract,
    Transport,
}

impl ProviderErrorClass {
    #[must_use]
    pub const fn is_retryable(self) -> bool {
        matches!(
            self,
            Self::Timeout | Self::Connection | Self::RateLimit | Self::Server
        )
    }

    #[must_use]
    pub const fn error_class(self) -> &'static str {
        match self {
            Self::Timeout => "provider_timeout",
            Self::Connection => "provider_connection",
            Self::RateLimit => "provider_rate_limit",
            Self::Quota => "provider_quota",
            Self::Server => "provider_server",
            Self::Authentication => "provider_authentication",
            Self::Permission => "provider_permission",
            Self::NotFound => "provider_not_found",
            Self::Request => "provider_request",
            Self::ResponseContract => "provider_response_contract",
            Self::Transport => "provider_transport",
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Timeout => "timeout",
            Self::Connection => "connection",
            Self::RateLimit => "rate limit",
            Self::Quota => "quota",
            Self::Server => "server",
            Self::Authentication => "authentication",
            Self::Permission => "permission",
            Self::NotFound => "not found",
            Self::Request => "request",
            Self::ResponseContract => "response contract",
            Self::Transport => "transport",
        }
    }
}

/// Sanitized, typed outcome of a bounded external provider request.
///
/// Remote response bodies and submitted context are intentionally absent.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{provider} provider {class_label} failure after {attempts} attempt(s)")]
pub struct ProviderError {
    provider: &'static str,
    class: ProviderErrorClass,
    class_label: &'static str,
    attempts: u32,
    upstream_status: Option<u16>,
}

impl ProviderError {
    #[must_use]
    pub const fn new(
        provider: &'static str,
        class: ProviderErrorClass,
        attempts: u32,
        upstream_status: Option<u16>,
    ) -> Self {
        Self {
            provider,
            class,
            class_label: class.label(),
            attempts: if attempts == 0 { 1 } else { attempts },
            upstream_status,
        }
    }

    #[must_use]
    pub const fn class(&self) -> ProviderErrorClass {
        self.class
    }

    #[must_use]
    pub const fn attempts(&self) -> u32 {
        self.attempts
    }

    #[must_use]
    pub const fn upstream_status(&self) -> Option<u16> {
        self.upstream_status
    }

    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        self.class.is_retryable()
    }

    #[must_use]
    pub const fn error_class(&self) -> &'static str {
        self.class.error_class()
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("authentication required")]
    Unauthorized,
    #[error("insufficient workspace permissions")]
    Forbidden,
    #[error("resource not found")]
    NotFound,
    #[error("invalid request: {0}")]
    Invalid(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("agent provider unavailable: {0}")]
    Agent(String),
    #[error(transparent)]
    Provider(#[from] ProviderError),
    #[error("external connector unavailable: {0}")]
    Connector(String),
    #[error("GitHub rate limit; retry after {retry_after_seconds} seconds")]
    ConnectorRateLimited { retry_after_seconds: u64 },
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let retry_after = match &self {
            Self::ConnectorRateLimited {
                retry_after_seconds,
            } => Some(*retry_after_seconds),
            _ => None,
        };
        let (status, code, message) = match self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized", self.to_string()),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden", self.to_string()),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found", self.to_string()),
            Self::Invalid(_) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_request",
                self.to_string(),
            ),
            Self::Conflict(_) => (StatusCode::CONFLICT, "conflict", self.to_string()),
            Self::Agent(_) => (
                StatusCode::BAD_GATEWAY,
                "agent_unavailable",
                self.to_string(),
            ),
            Self::Provider(error) => (
                if error.is_retryable() {
                    StatusCode::SERVICE_UNAVAILABLE
                } else {
                    StatusCode::BAD_GATEWAY
                },
                "agent_unavailable",
                error.to_string(),
            ),
            Self::ConnectorRateLimited { .. } => (
                StatusCode::SERVICE_UNAVAILABLE,
                "connector_rate_limited",
                self.to_string(),
            ),
            Self::Connector(_) => (
                StatusCode::BAD_GATEWAY,
                "connector_unavailable",
                self.to_string(),
            ),
            Self::Database(error) => {
                tracing::error!(?error, "database operation failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "database_error",
                    "database operation failed".into(),
                )
            }
            Self::Internal(message) => {
                tracing::error!(%message, "internal operation failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "internal operation failed".into(),
                )
            }
        };
        let mut response = (status, Json(ErrorBody { code, message })).into_response();
        if let Some(seconds) = retry_after
            && let Ok(value) = seconds.to_string().parse()
        {
            response
                .headers_mut()
                .insert(axum::http::header::RETRY_AFTER, value);
        }
        response
    }
}

impl AppError {
    #[must_use]
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Invalid(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::ConnectorRateLimited { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::Provider(error) if error.is_retryable() => StatusCode::SERVICE_UNAVAILABLE,
            Self::Agent(_) | Self::Provider(_) | Self::Connector(_) => StatusCode::BAD_GATEWAY,
            Self::Database(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    #[must_use]
    pub fn public_code(&self) -> &'static str {
        match self {
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "forbidden",
            Self::NotFound => "not_found",
            Self::Conflict(_) => "conflict",
            Self::Invalid(_) => "invalid_request",
            Self::Agent(_) | Self::Provider(_) => "agent_unavailable",
            Self::Connector(_) => "connector_unavailable",
            Self::ConnectorRateLimited { .. } => "connector_rate_limited",
            Self::Database(_) => "database_error",
            Self::Internal(_) => "internal_error",
        }
    }

    #[must_use]
    pub fn public_message(&self) -> String {
        match self {
            Self::Database(_) => "database operation failed".into(),
            Self::Internal(_) => "internal operation failed".into(),
            _ => self.to_string(),
        }
    }

    /// Whether a durable command may safely call the AI provider again.
    ///
    /// Only failures already classified from timeout/connect/rate-limit
    /// 429/5xx are resumable. Quota exhaustion, agent validation, and provider
    /// contract failures are final.
    #[must_use]
    pub const fn is_retryable_provider_failure(&self) -> bool {
        matches!(self, Self::Provider(error) if error.is_retryable())
    }

    #[must_use]
    pub const fn provider_attempts(&self) -> Option<u32> {
        match self {
            Self::Provider(error) => Some(error.attempts()),
            _ => None,
        }
    }

    #[must_use]
    pub fn model_run_error_class(&self) -> &'static str {
        match self {
            Self::Provider(error) => error.error_class(),
            _ => self.public_code(),
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
