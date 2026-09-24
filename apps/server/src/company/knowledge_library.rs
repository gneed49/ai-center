//! Paged knowledge discovery independently of the deliberately bounded graph.
use crate::{
    error::{AppError, AppResult},
    service::AppState,
};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Default, Deserialize)]
pub struct Query {
    pub project_id: Option<Uuid>,
    pub q: Option<String>,
    pub history: Option<bool>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Lists exact knowledge versions, including archived scopes, under tenant RLS.
///
/// # Errors
/// Refuses foreign project identities, oversized queries and invalid pagination.
pub async fn list(state: &AppState, query: Query) -> AppResult<Value> {
    let limit = query.limit.unwrap_or(25);
    let offset = query.offset.unwrap_or(0);
    let search = query.q.as_deref().unwrap_or("").trim();
    if !(1..=100).contains(&limit) || offset > 1_000_000 || search.chars().count() > 200 {
        return Err(AppError::Invalid(
            "La recherche est limitée à 200 caractères et 100 résultats par page.".into(),
        ));
    }
    let mut tx = state.begin_request().await?;
    if let Some(project) = query.project_id {
        let exists:bool=sqlx::query_scalar("select exists(select 1 from app.projects where public_id=$1 and workspace_id=app.current_workspace_id())")
            .bind(project).fetch_one(&mut *tx).await?;
        if !exists {
            return Err(AppError::NotFound);
        }
    }
    let result = sqlx::query_scalar(include_str!("knowledge_library.sql"))
        .bind(query.project_id)
        .bind(search)
        .bind(query.history.unwrap_or(false))
        .bind(i64::from(limit))
        .bind(i64::from(offset))
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(result)
}
