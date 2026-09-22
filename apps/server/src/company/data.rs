//! Owner data portability and reversible project archival.
use super::{audit, complete, editor, export_columns::TABLES, owner};
use crate::{
    error::{AppError, AppResult},
    idempotency::IdempotencyLease,
    models::ProjectSummary,
    service::AppState,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use std::collections::BTreeMap;
use uuid::Uuid;

const MAX_EXPORT_ROWS: i64 = 25_000;
const MAX_EXPORT_BYTES: i64 = 32 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenameProject {
    pub name: String,
    pub expected_updated_at: chrono::DateTime<chrono::Utc>,
}

/// Renames a project using the exact last-seen revision timestamp.
/// # Errors
/// Rejects insufficient rights, invalid names, archives, foreign scopes or stale edits.
pub async fn rename(
    state: &AppState,
    id: Uuid,
    input: RenameProject,
    lease: Option<&IdempotencyLease>,
) -> AppResult<ProjectSummary> {
    editor(state)?;
    let name = input.name.trim();
    if !(1..=120).contains(&name.chars().count()) || name.chars().any(char::is_control) {
        return Err(AppError::Invalid(
            "Le nom du projet doit contenir entre 1 et 120 caractères, sans caractère de contrôle."
                .into(),
        ));
    }
    let mut tx = state.begin_request().await?;
    let (internal,kind,current):(i64,String,chrono::DateTime<chrono::Utc>)=sqlx::query_as("select id,scope_kind,updated_at from app.projects where public_id=$1 and workspace_id=app.current_workspace_id() for update")
        .bind(id).fetch_optional(&mut *tx).await?.ok_or(AppError::NotFound)?;
    if kind != "project" {
        return Err(AppError::Invalid(
            "Renommez la société depuis ses réglages.".into(),
        ));
    }
    require_active(&mut tx, internal).await?;
    if current != input.expected_updated_at {
        return Err(AppError::Conflict(
            "Le projet a changé. Rechargez-le avant de le renommer.".into(),
        ));
    }
    sqlx::query(
        "update app.projects set name=$2,graph_version=graph_version+1 where id=$1 and name<>$2",
    )
    .bind(internal)
    .bind(name)
    .execute(&mut *tx)
    .await?;
    audit(
        &mut tx,
        state.actor_id,
        "project.renamed",
        "project",
        id,
        json!({"name":name}),
    )
    .await?;
    let result=sqlx::query_as("select public_id,name,objective,summary,status,graph_version,created_at,updated_at from app.projects where id=$1").bind(internal).fetch_one(&mut *tx).await?;
    complete(&mut tx, lease, &result).await?;
    tx.commit().await?;
    Ok(result)
}

#[derive(Serialize)]
pub struct CompanyDataExport {
    pub format: &'static str,
    pub workspace_public_id: Uuid,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub complete: bool,
    pub record_count: i64,
    pub data: BTreeMap<String, Vec<Value>>,
    pub excluded: Vec<&'static str>,
}

/// Exports only explicitly selected business columns from one owner scope.
///
/// # Errors
/// Rejects non-owners, missing membership, oversized exports and database errors.
pub async fn export(state: &AppState) -> AppResult<CompanyDataExport> {
    owner(state)?;
    let workspace = state.workspace_internal_id.ok_or(AppError::Forbidden)?;
    let mut tx = state.pool.begin().await?;
    sqlx::query("set transaction isolation level repeatable read, read only")
        .execute(&mut *tx)
        .await?;
    sqlx::query("select set_config('app.current_actor_id',$1,true),set_config('app.current_workspace_id',$2,true),set_config('app.current_workspace_role',$3,true)")
        .bind(state.actor_id.to_string()).bind(workspace.to_string()).bind(&state.workspace_role).execute(&mut *tx).await?;
    let authorized: bool = sqlx::query_scalar(
        "select app.has_workspace_role(app.current_workspace_id(),array['owner'])",
    )
    .fetch_one(&mut *tx)
    .await?;
    if !authorized {
        return Err(AppError::Forbidden);
    }
    let generated_at = sqlx::query_scalar("select transaction_timestamp()")
        .fetch_one(&mut *tx)
        .await?;
    let mut records = 0_i64;
    let mut size = 0_i64;
    let mut data = BTreeMap::new();
    for &(table, columns) in TABLES {
        let scope_column = if table == "workspaces" {
            "id"
        } else {
            "workspace_id"
        };
        // Identifiers are compile-time allowlists, never request input. Count
        // bytes before fetching so oversized histories cannot fill app memory.
        let selection = format!(
            "select {columns} from app.{table} where {scope_column}=app.current_workspace_id() order by id limit $1"
        );
        let (count,bytes):(i64,i64)=sqlx::query_as(&format!("select count(*),coalesce(sum(octet_length(to_jsonb(exported)::text)),0)::bigint from ({selection}) exported"))
            .bind(MAX_EXPORT_ROWS-records+1).fetch_one(&mut *tx).await?;
        records = records.saturating_add(count);
        size = size.saturating_add(bytes);
        if records > MAX_EXPORT_ROWS || size > MAX_EXPORT_BYTES {
            return Err(AppError::Invalid("Company export exceeds 25000 records or 32 MiB; request the controlled operator export procedure. No partial export was produced.".into()));
        }
        let rows: Vec<Value> = sqlx::query_scalar(&format!(
            "select to_jsonb(exported) from ({selection}) exported"
        ))
        .bind(count)
        .fetch_all(&mut *tx)
        .await?;
        data.insert(table.into(), rows);
    }
    tx.commit().await?;
    Ok(CompanyDataExport {
        format: "ai-center-company-data-v1",
        workspace_public_id: state.workspace_id,
        generated_at,
        complete: true,
        record_count: records,
        data,
        excluded: vec![
            "credentials and encrypted envelopes",
            "provider subscription sessions",
            "connection configuration and secret references",
            "invitation tokens and token digests",
            "idempotency and active worker lease capabilities",
            "remote files that have not been observed or imported",
        ],
    })
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveProject {
    pub archived: bool,
}

/// Lists archived projects so the owner can inspect or restore their history.
/// # Errors
/// Rejects non-owners or a database error.
pub async fn archived_projects(state: &AppState) -> AppResult<Vec<ProjectSummary>> {
    owner(state)?;
    let mut tx = state.begin_request().await?;
    let authorized: bool = sqlx::query_scalar(
        "select app.has_workspace_role(app.current_workspace_id(),array['owner'])",
    )
    .fetch_one(&mut *tx)
    .await?;
    if !authorized {
        return Err(AppError::Forbidden);
    }
    let projects = sqlx::query_as("select public_id,name,objective,summary,status,graph_version,created_at,updated_at from app.projects where workspace_id=app.current_workspace_id() and scope_kind='project' and status='archived' order by updated_at desc,public_id")
        .fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(projects)
}

/// Archives/restores a project and invalidates exact dependent projections.
///
/// # Errors
/// Rejects non-owners, unknown projects, the company container and database errors.
pub async fn archive(
    state: &AppState,
    id: Uuid,
    input: ArchiveProject,
    lease: Option<&IdempotencyLease>,
) -> AppResult<ProjectSummary> {
    owner(state)?;
    let mut tx = state.begin_request().await?;
    let (internal,kind,current):(i64,String,String)=sqlx::query_as("select id,scope_kind,status from app.projects where public_id=$1 and workspace_id=app.current_workspace_id() for update")
        .bind(id).fetch_optional(&mut *tx).await?.ok_or(AppError::NotFound)?;
    if kind == "company" {
        return Err(AppError::Invalid(
            "The company context cannot be archived as a project".into(),
        ));
    }
    let next = if input.archived { "archived" } else { "active" };
    if current != next {
        sqlx::query("update app.projects set status=$2,graph_version=graph_version+1,updated_at=now() where id=$1")
            .bind(internal).bind(next).execute(&mut *tx).await?;
        if input.archived {
            sqlx::query("with changed as (update app.context_packs p set status='stale',invalidated_at=now(),stale_reason='A source project was archived'
              where p.status='current' and (p.project_id=$1 or exists(select 1 from app.context_pack_scope_sources s where s.context_pack_id=p.id and s.source_project_id=$1 and s.decision='included')) returning p.id)
              update app.deliverables d set status='stale',stale_at=now() where d.status in ('draft','committed') and (d.project_id=$1 or d.source_context_pack_id in(select id from changed))")
                .bind(internal).execute(&mut *tx).await?;
        }
        audit(
            &mut tx,
            state.actor_id,
            if input.archived {
                "project.archived"
            } else {
                "project.restored"
            },
            "project",
            id,
            json!({"status":next}),
        )
        .await?;
    }
    let result:ProjectSummary=sqlx::query_as("select public_id,name,objective,summary,status,graph_version,created_at,updated_at from app.projects where id=$1")
        .bind(internal).fetch_one(&mut *tx).await?;
    complete(&mut tx, lease, &result).await?;
    tx.commit().await?;
    Ok(result)
}

/// Service guard shared by user commands and background producers.
/// Reads of archived history remain allowed.
pub(crate) async fn require_active(
    tx: &mut Transaction<'_, Postgres>,
    project: i64,
) -> AppResult<()> {
    let status:Option<String>=sqlx::query_scalar("select status from app.projects where id=$1 and workspace_id=app.current_workspace_id() for update")
        .bind(project).fetch_optional(&mut **tx).await?;
    if status.as_deref() == Some("active") {
        Ok(())
    } else {
        Err(AppError::Conflict("This project is archived or unavailable; restore it before creating or modifying context".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn export_uses_explicit_business_columns_without_credentials_or_capabilities() {
        for (table, columns) in TABLES {
            assert!(!matches!(
                *table,
                "provider_connections"
                    | "work_tool_connections"
                    | "tool_connections"
                    | "workspace_invitations"
                    | "idempotency_records"
            ));
            for column in columns.split(',') {
                for forbidden in [
                    "secret",
                    "credential",
                    "encrypted",
                    "token_hash",
                    "lease_token",
                    "bootstrap_request_hash",
                ] {
                    assert!(
                        !column.contains(forbidden),
                        "exported capability field {table}.{column}"
                    );
                }
            }
        }
        assert!(
            TABLES
                .iter()
                .any(|(table, _)| *table == "knowledge_entry_versions")
        );
        assert!(
            TABLES
                .iter()
                .any(|(table, _)| *table == "artifact_document_versions")
        );
    }
}
