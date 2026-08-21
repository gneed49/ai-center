use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("resource not found")]
    NotFound,
    #[error("invalid request: {0}")]
    Invalid(String),
    #[error("agent provider unavailable: {0}")]
    Agent(String),
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
        let (status, code, message) = match self {
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found", self.to_string()),
            Self::Invalid(_) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_request",
                self.to_string(),
            ),
            Self::Agent(_) => (
                StatusCode::BAD_GATEWAY,
                "agent_unavailable",
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
        (status, Json(ErrorBody { code, message })).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
