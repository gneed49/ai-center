//! Company administration uses the same scoped transactions, RLS and durable
//! command records as projects. It never calls a provider or sends invitations.
mod graph;
pub mod graph_source;
pub mod knowledge_library;
pub mod models;

pub mod data;
mod export_columns;
pub mod project_data;

pub use graph::{create_edge, graph};
use models::{
    AgentSummary, CompanyInput, CompanyMember, CompanyOverview, CompanyWorkspace, CreateCompany,
    ScopeRef, UpdateMember,
};
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    auth::RequestContext,
    error::{AppError, AppResult},
    idempotency::{self, IdempotencyLease},
    service::AppState,
};

fn owner(state: &AppState) -> AppResult<()> {
    if state.workspace_role == "owner" {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

pub(crate) fn editor(state: &AppState) -> AppResult<()> {
    if matches!(state.workspace_role.as_str(), "owner" | "editor") {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

fn validate_company(name: &str, description: &str) -> AppResult<()> {
    if !(1..=120).contains(&name.trim().chars().count()) || description.chars().count() > 4000 {
        return Err(AppError::Invalid(
            "Company name must contain 1–120 characters and description at most 4000".into(),
        ));
    }
    Ok(())
}

pub(crate) fn database_error(error: sqlx::Error) -> AppError {
    match error
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code)
        .as_deref()
    {
        Some("42501") => AppError::Forbidden,
        Some("23505") => {
            AppError::Conflict("This identity is already bound to another command".into())
        }
        Some("23503") => {
            AppError::Invalid("The referenced object is outside the authorized scope".into())
        }
        Some("22023" | "23514") => AppError::Invalid("Invalid company or graph data".into()),
        Some("54000") => {
            AppError::Invalid("The limit of ten companies per account has been reached".into())
        }
        _ => error.into(),
    }
}

pub(crate) async fn complete<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    lease: Option<&IdempotencyLease>,
    output: &T,
) -> AppResult<()> {
    if let Some(lease) = lease {
        let value =
            serde_json::to_value(output).map_err(|error| AppError::Internal(error.to_string()))?;
        idempotency::complete(tx, lease, 200, value).await?;
    }
    Ok(())
}

pub(crate) async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    action: &str,
    kind: &str,
    id: Uuid,
    details: Value,
) -> AppResult<()> {
    sqlx::query("insert into app.audit_events(workspace_id,actor_id,action,object_kind,object_public_id,after_state) values(app.current_workspace_id(),$1,$2,$3,$4,$5)")
        .bind(actor).bind(action).bind(kind).bind(id).bind(details).execute(&mut **tx).await?;
    Ok(())
}

/// Creates a company for a verified actor, without trusting a selected workspace.
///
/// # Errors
/// Rejects invalid inputs, reused identities, creation limits and database failures.
pub async fn create(state: &AppState, input: CreateCompany) -> AppResult<CompanyOverview> {
    validate_company(&input.name, &input.description)?;
    if input.public_id.is_nil() || state.actor_id.is_nil() {
        return Err(AppError::Invalid("Non-nil identities are required".into()));
    }
    let request_hash = idempotency::hash_request(&input)?;
    let mut tx = state.pool.begin().await?;
    sqlx::query("select set_config('app.current_actor_id',$1,true)")
        .bind(state.actor_id.to_string())
        .execute(&mut *tx)
        .await?;
    let public_id: Uuid = sqlx::query_scalar("select app.create_company_workspace($1,$2,$3,$4)")
        .bind(input.public_id)
        .bind(input.name.trim())
        .bind(&input.description)
        .bind(request_hash)
        .fetch_one(&mut *tx)
        .await
        .map_err(database_error)?;
    let (internal_id, role): (i64, String) =
        sqlx::query_as("select workspace_id,role from app.authorize_workspace_member($1,$2)")
            .bind(public_id)
            .bind(state.actor_id)
            .fetch_one(&mut *tx)
            .await?;
    tx.commit().await?;
    overview(&state.scoped(&RequestContext {
        actor_id: state.actor_id,
        workspace_id: public_id,
        workspace_internal_id: Some(internal_id),
        workspace_role: role,
    }))
    .await
}

/// Reads the caller's accepted company membership and its visible context.
///
/// # Errors
/// Returns not found or a database error if the scoped company is unavailable.
pub async fn overview(state: &AppState) -> AppResult<CompanyOverview> {
    let mut tx = state.begin_request().await?;
    let result = overview_in_transaction(state, &mut tx).await?;
    tx.commit().await?;
    Ok(result)
}

async fn overview_in_transaction(
    state: &AppState,
    tx: &mut Transaction<'_, Postgres>,
) -> AppResult<CompanyOverview> {
    let workspace:CompanyWorkspace=sqlx::query_as("select public_id,name,description,app.current_workspace_role() as role from app.workspaces where public_id=$1")
        .bind(state.workspace_id).fetch_optional(&mut **tx).await?.ok_or(AppError::NotFound)?;
    let company_id:Option<Uuid>=sqlx::query_scalar("select public_id from app.projects where workspace_id=app.current_workspace_id() and scope_kind='company'")
        .fetch_optional(&mut **tx).await?;
    let agents = agents_in_transaction(tx).await?;
    let members = members_in_transaction(tx).await?;
    let projects=sqlx::query_as("select public_id,name,objective,summary,status,graph_version,created_at,updated_at from app.projects
        where workspace_id=app.current_workspace_id() and scope_kind='project' and status='active' order by updated_at desc,id desc")
        .fetch_all(&mut **tx).await?;
    Ok(CompanyOverview {
        workspace,
        company_scope: company_id.map(|project_public_id| ScopeRef {
            kind: "company".into(),
            project_public_id,
        }),
        agents,
        projects,
        members,
        setup_complete: company_id.is_some(),
    })
}

async fn agents_in_transaction(tx: &mut Transaction<'_, Postgres>) -> AppResult<Vec<AgentSummary>> {
    Ok(sqlx::query_as("select n.public_id as node_public_id,p.public_id as project_public_id,p.scope_kind,n.node_key,
        a.profile_key,n.title as name,case n.node_key when 'product' then 'product_manager' when 'tech' then 'tech_lead'
        when 'sales' then 'sales' when 'dev' then 'developer' else 'general' end as role,
        (a.scope_kind='tech') as requires_context_pack
        from app.context_nodes n join app.projects p on p.id=n.project_id join app.agent_profiles a on a.id=n.agent_profile_id
        where n.workspace_id=app.current_workspace_id() and p.status='active' order by p.scope_kind,n.node_key,p.id")
        .fetch_all(&mut **tx).await?)
}

async fn members_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
) -> AppResult<Vec<CompanyMember>> {
    Ok(sqlx::query_as(
        "select public_id,actor_id,role,invitation_status,accepted_at from app.workspace_members
        where workspace_id=app.current_workspace_id() order by created_at,id",
    )
    .fetch_all(&mut **tx)
    .await?)
}

/// Lists company memberships allowed by the caller's RLS context.
///
/// # Errors
/// Returns a database error when the membership query cannot complete.
pub async fn members(state: &AppState) -> AppResult<Vec<CompanyMember>> {
    let mut tx = state.begin_request().await?;
    let rows = members_in_transaction(&mut tx).await?;
    tx.commit().await?;
    Ok(rows)
}

/// Updates company metadata and optionally initializes its typed context scope.
///
/// # Errors
/// Rejects non-owners, invalid input and failed database or command transactions.
pub async fn update(
    state: &AppState,
    input: CompanyInput,
    setup: bool,
    lease: Option<&IdempotencyLease>,
) -> AppResult<CompanyOverview> {
    owner(state)?;
    validate_company(&input.name, &input.description)?;
    let mut tx = state.begin_request().await?;
    let workspace_id: i64 =
        sqlx::query_scalar("select id from app.workspaces where public_id=$1 for update")
            .bind(state.workspace_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(AppError::NotFound)?;
    sqlx::query("update app.workspaces set name=$2,description=$3 where id=$1")
        .bind(workspace_id)
        .bind(input.name.trim())
        .bind(&input.description)
        .execute(&mut *tx)
        .await?;
    if setup {
        let project_id:Uuid=sqlx::query_scalar("insert into app.projects(workspace_id,template_id,name,objective,scope_kind,created_by_actor_id)
            select $1,id,'Contexte société',$2,'company',$3 from app.project_templates where template_key='software-product-delivery'
            on conflict(workspace_id) where scope_kind='company' do update set scope_kind='company' returning public_id")
            .bind(workspace_id).bind(&input.description).bind(state.actor_id).fetch_one(&mut *tx).await?;
        sqlx::query("select app.ensure_scope_agents($1)")
            .bind(project_id)
            .execute(&mut *tx)
            .await
            .map_err(database_error)?;
        // Upgrade existing customer projects without replacing their nodes or instructions.
        let projects:Vec<Uuid>=sqlx::query_scalar("select public_id from app.projects where workspace_id=$1 and scope_kind='project' and status='active'")
            .bind(workspace_id).fetch_all(&mut *tx).await?;
        for project in projects {
            sqlx::query("select app.ensure_scope_agents($1)")
                .bind(project)
                .execute(&mut *tx)
                .await
                .map_err(database_error)?;
        }
    }
    audit(
        &mut tx,
        state.actor_id,
        if setup {
            "company.configured"
        } else {
            "company.updated"
        },
        "workspace",
        state.workspace_id,
        json!({"name":input.name.trim(),"description":input.description}),
    )
    .await?;
    let result = overview_in_transaction(state, &mut tx).await?;
    complete(&mut tx, lease, &result).await?;
    tx.commit().await?;
    Ok(result)
}

fn validate_member_change(input: &UpdateMember) -> AppResult<()> {
    if input.role.is_none() && input.invitation_status.is_none() {
        return Err(AppError::Invalid("A member change is required".into()));
    }
    if input
        .role
        .as_deref()
        .is_some_and(|role| !matches!(role, "owner" | "editor" | "viewer"))
        || input
            .invitation_status
            .as_deref()
            .is_some_and(|status| !matches!(status, "accepted" | "revoked"))
    {
        return Err(AppError::Invalid(
            "Unsupported member role or status".into(),
        ));
    }
    Ok(())
}

/// Changes a membership while retaining at least one accepted owner.
///
/// # Errors
/// Rejects non-owners, unknown members, invalid transitions and last-owner removal.
pub async fn update_member(
    state: &AppState,
    member_id: Uuid,
    input: UpdateMember,
    lease: Option<&IdempotencyLease>,
) -> AppResult<CompanyMember> {
    owner(state)?;
    validate_member_change(&input)?;
    let mut tx = state.begin_request().await?;
    // Serialize all changes to ownership in this company, not only one member.
    sqlx::query("select id from app.workspaces where public_id=$1 for update")
        .bind(state.workspace_id)
        .execute(&mut *tx)
        .await?;
    let existing: CompanyMember = sqlx::query_as(
        "select public_id,actor_id,role,invitation_status,accepted_at from app.workspace_members
        where public_id=$1 and workspace_id=app.current_workspace_id() for update",
    )
    .bind(member_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    let role = input.role.as_deref().unwrap_or(&existing.role);
    let status = input
        .invitation_status
        .as_deref()
        .unwrap_or(&existing.invitation_status);
    if existing.role == "owner"
        && existing.invitation_status == "accepted"
        && (role != "owner" || status != "accepted")
    {
        let count:i64=sqlx::query_scalar("select count(*) from app.workspace_members where workspace_id=app.current_workspace_id() and role='owner' and invitation_status='accepted'")
            .fetch_one(&mut *tx).await?;
        if count <= 1 {
            return Err(AppError::Conflict(
                "The last accepted owner cannot be removed".into(),
            ));
        }
    }
    if existing.actor_id == state.actor_id && (role != "owner" || status != "accepted") {
        return Err(AppError::Conflict("Another owner must change your own role or revoke your access after ownership has been transferred".into()));
    }
    audit(
        &mut tx,
        state.actor_id,
        "company.member_updated",
        "workspace_member",
        member_id,
        json!({"role":role,"invitation_status":status}),
    )
    .await?;
    let result: CompanyMember = sqlx::query_as(
        "update app.workspace_members set role=$2,invitation_status=$3,
        accepted_at=case when $3='accepted' then coalesce(accepted_at,now()) else accepted_at end
        where public_id=$1 and workspace_id=app.current_workspace_id()
        returning public_id,actor_id,role,invitation_status,accepted_at",
    )
    .bind(member_id)
    .bind(role)
    .bind(status)
    .fetch_one(&mut *tx)
    .await
    .map_err(database_error)?;
    complete(&mut tx, lease, &result).await?;
    tx.commit().await?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn company_input_is_bounded_in_unicode_characters() {
        assert!(validate_company("   ", "").is_err());
        assert!(validate_company(&"é".repeat(120), "").is_ok());
        assert!(validate_company(&"a".repeat(121), "").is_err());
        assert!(validate_company("Société", &"a".repeat(4001)).is_err());
    }
    #[test]
    fn member_changes_do_not_accept_invented_privileges_or_implicit_invitations() {
        for role in ["admin", "member", "service_role"] {
            assert!(
                validate_member_change(&UpdateMember {
                    role: Some(role.into()),
                    invitation_status: None
                })
                .is_err()
            );
        }
        assert!(
            validate_member_change(&UpdateMember {
                role: None,
                invitation_status: None
            })
            .is_err()
        );
        assert!(
            validate_member_change(&UpdateMember {
                role: None,
                invitation_status: Some("pending".into())
            })
            .is_err()
        );
        assert!(
            validate_member_change(&UpdateMember {
                role: Some("viewer".into()),
                invitation_status: Some("revoked".into())
            })
            .is_ok()
        );
    }
}
