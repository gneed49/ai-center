//! Project portability without expanding to unrelated conversations or credentials.
use super::{export_columns::TABLES, owner};
use crate::{
    error::{AppError, AppResult},
    service::AppState,
};
use serde::Serialize;
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use std::collections::BTreeMap;
use uuid::Uuid;

const MAX_ROWS: i64 = 25_000;
const MAX_BYTES: i64 = 32 * 1024 * 1024;

#[derive(Serialize)]
pub struct ProjectDataExport {
    pub format: &'static str,
    pub workspace_public_id: Uuid,
    pub project_public_id: Uuid,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub complete: bool,
    pub record_count: i64,
    pub data: BTreeMap<String, Vec<Value>>,
    pub referenced_sources: Vec<Value>,
    pub source_expansion: &'static str,
    pub excluded: Vec<&'static str>,
}

/// Produces one coherent owner export, including exact directly cited sources.
/// # Errors
/// Refuses another tenant, the company container, insufficient rights or oversized output.
pub async fn export(state: &AppState, project: Uuid) -> AppResult<ProjectDataExport> {
    owner(state)?;
    let mut tx = state.pool.begin().await?;
    sqlx::query("set transaction isolation level repeatable read, read only")
        .execute(&mut *tx)
        .await?;
    sqlx::query("select set_config('app.current_actor_id',$1,true),set_config('app.current_workspace_id',$2,true),set_config('app.current_workspace_role',$3,true)")
        .bind(state.actor_id.to_string()).bind(state.workspace_internal_id.ok_or(AppError::Forbidden)?.to_string()).bind(&state.workspace_role).execute(&mut *tx).await?;
    let authorized: bool = sqlx::query_scalar(
        "select app.has_workspace_role(app.current_workspace_id(),array['owner'])",
    )
    .fetch_one(&mut *tx)
    .await?;
    if !authorized {
        return Err(AppError::Forbidden);
    }
    let (internal,kind):(i64,String)=sqlx::query_as("select id,scope_kind from app.projects where public_id=$1 and workspace_id=app.current_workspace_id()")
        .bind(project).fetch_optional(&mut *tx).await?.ok_or(AppError::NotFound)?;
    if kind != "project" {
        return Err(AppError::Invalid(
            "Utilisez l’export société pour ce contexte.".into(),
        ));
    }
    let generated_at = sqlx::query_scalar("select transaction_timestamp()")
        .fetch_one(&mut *tx)
        .await?;
    let mut data = BTreeMap::new();
    let mut budget = Budget::default();
    for &(table, columns) in TABLES {
        let Some(predicate) = owned_predicate(table) else {
            continue;
        };
        let selection = format!(
            "select {columns} from app.{table} t where t.workspace_id=app.current_workspace_id() and ({predicate}) order by id"
        );
        let rows = read_bounded(&mut tx, &selection, internal, &mut budget).await?;
        data.insert(table.into(), rows);
    }
    let referenced_sources = read_bounded(
        &mut tx,
        include_str!("project_export_sources.sql"),
        internal,
        &mut budget,
    )
    .await?;
    tx.commit().await?;
    Ok(ProjectDataExport {
        format: "ai-center-project-data-v1",
        workspace_public_id: state.workspace_id,
        project_public_id: project,
        generated_at,
        complete: true,
        record_count: budget.rows,
        data,
        referenced_sources,
        source_expansion: "direct_exact_versions_and_captured_origin_receipts",
        excluded: vec![
            "credentials, connection configuration, invitations and worker capabilities",
            "unrelated projects and their conversations",
            "recursive expansion of referenced documents",
            "remote files that were never observed or imported",
            "global catalogs and authentication accounts",
        ],
    })
}

fn owned_predicate(table: &str) -> Option<&'static str> {
    match table {
        "ai_call_reservations"
        | "steward_scan_progress"
        | "workspace_members"
        | "workspace_automation_controls"
        | "workspaces" => None,
        "projects" => Some("t.id=$1"),
        "artifact_version_sources" => Some(
            "exists(select 1 from app.artifact_document_versions v where v.id=t.version_id and v.project_id=$1)",
        ),
        "publication_observations" => Some(
            "exists(select 1 from app.publication_jobs j where j.id=t.publication_job_id and j.project_id=$1)",
        ),
        "edges" => Some("t.project_id=$1 or t.target_project_id=$1"),
        "audit_events" => Some(
            "t.project_id=$1 or (t.project_id is null and t.object_kind='project' and t.object_public_id=(select public_id from app.projects where id=$1))",
        ),
        _ => Some("t.project_id=$1"),
    }
}

#[derive(Default)]
struct Budget {
    rows: i64,
    bytes: i64,
}

async fn read_bounded(
    tx: &mut Transaction<'_, Postgres>,
    selection: &str,
    project: i64,
    budget: &mut Budget,
) -> AppResult<Vec<Value>> {
    let bounded = format!("{selection} limit $2");
    let (rows,bytes):(i64,i64)=sqlx::query_as(&format!("select count(*),coalesce(sum(octet_length(to_jsonb(item)::text)),0)::bigint from ({bounded}) item"))
        .bind(project).bind(MAX_ROWS-budget.rows+1).fetch_one(&mut **tx).await?;
    budget.rows += rows;
    budget.bytes += bytes;
    if budget.rows > MAX_ROWS || budget.bytes > MAX_BYTES {
        return Err(AppError::Invalid("L’export dépasse 25 000 lignes ou 32 Mio. Aucun export partiel n’a été produit ; une intervention opérateur est nécessaire.".into()));
    }
    Ok(
        sqlx::query_scalar(&format!("select to_jsonb(item) from ({bounded}) item"))
            .bind(project)
            .bind(rows)
            .fetch_all(&mut **tx)
            .await?,
    )
}
