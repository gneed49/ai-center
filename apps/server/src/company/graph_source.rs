//! Exact read-only graph objects. UUIDs are resolved with both project and tenant
//! identity; historical sources are never substituted with a current version.
use crate::{
    error::{AppError, AppResult},
    service::AppState,
};
use serde_json::Value;
use uuid::Uuid;

/// Reads an explicitly supported business object under the caller's RLS context.
///
/// # Errors
/// Unknown types/identities, foreign projects and revoked memberships are refused.
pub async fn read(state: &AppState, project: Uuid, kind: &str, id: Uuid) -> AppResult<Value> {
    if !matches!(kind, "knowledge" | "context_pack" | "task" | "artifact") {
        return Err(AppError::NotFound);
    }
    let mut tx = state.begin_request().await?;
    let result: Value = sqlx::query_scalar(include_str!("graph_source.sql"))
        .bind(project)
        .bind(kind)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::NotFound)?;
    tx.commit().await?;
    Ok(result)
}
