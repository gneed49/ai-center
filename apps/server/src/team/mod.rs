//! Company invitations separate possession of a link, verified identity and
//! explicit acceptance. Only digests and non-sensitive receipts are persisted.
pub mod models;
pub use models::*;

use crate::{
    error::{AppError, AppResult},
    idempotency::IdempotencyLease,
    models::WorkspaceSummary,
    service::AppState,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const INVITATION_COLUMNS: &str = "public_id,role,label,case when status='pending' and expires_at<=now() then 'expired' else status end as status,expires_at,created_at,accepted_at";
const MEMBER_COLUMNS: &str = "public_id,actor_id,role,invitation_status,accepted_at,display_name";

fn owner(state: &AppState) -> AppResult<()> {
    if state.workspace_role == "owner" {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

fn error(error: sqlx::Error) -> AppError {
    match error
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code)
        .as_deref()
    {
        Some("P0002") => AppError::NotFound,
        Some("42501") => AppError::Forbidden,
        Some("22023" | "23514") => {
            AppError::Invalid("Les informations de l’invitation sont invalides.".into())
        }
        Some("23505") => {
            AppError::Conflict("Cette invitation appartient déjà à une autre demande.".into())
        }
        _ => error.into(),
    }
}

fn valid_name(name: &str) -> AppResult<()> {
    if !(1..=80).contains(&name.trim().chars().count()) || name.chars().any(char::is_control) {
        return Err(AppError::Invalid(
            "Indiquez un nom de 1 à 80 caractères.".into(),
        ));
    }
    Ok(())
}

fn token_hash(token: &str) -> AppResult<String> {
    if token.len() != 64
        || !token
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(AppError::NotFound);
    }
    Ok(format!("{:x}", Sha256::digest(token.as_bytes())))
}

fn validate_invitation(input: &CreateInvitation) -> AppResult<()> {
    if input.public_id.is_nil()
        || !matches!(input.role.as_str(), "editor" | "viewer")
        || !(1..=7).contains(&input.expires_in_days)
        || input.label.chars().count() > 120
        || input.label.chars().any(char::is_control)
        || input.token_hash.len() != 64
        || !input
            .token_hash
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(AppError::Invalid(
            "Choisissez un rôle, un libellé court et une durée de 1 à 7 jours.".into(),
        ));
    }
    Ok(())
}

/// Lists names and roles only inside the accepted company scope.
/// # Errors
/// Returns an authorization or database error.
pub async fn members(state: &AppState) -> AppResult<Vec<TeamMember>> {
    let mut tx = state.begin_request().await?;
    let rows=sqlx::query_as(&format!("select {MEMBER_COLUMNS} from app.workspace_members where workspace_id=app.current_workspace_id() order by accepted_at nulls last,id"))
        .fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(rows)
}

/// Changes only the calling member's presentation name, including for readers.
/// # Errors
/// Returns invalid input, withdrawn membership or database failure.
pub async fn profile(state: &AppState, input: SetProfile) -> AppResult<TeamMember> {
    valid_name(&input.display_name)?;
    let mut tx = state.begin_request().await?;
    let id: Uuid = sqlx::query_scalar("select app.set_member_display_name($1)")
        .bind(input.display_name.trim())
        .fetch_one(&mut *tx)
        .await
        .map_err(error)?;
    let row = sqlx::query_as(&format!(
        "select {MEMBER_COLUMNS} from app.workspace_members where public_id=$1"
    ))
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(row)
}

/// Lists the company's latest invitations without their digests or link secrets.
/// # Errors
/// Returns an authorization or database error.
pub async fn invitations(state: &AppState) -> AppResult<Vec<Invitation>> {
    owner(state)?;
    let mut tx = state.begin_request().await?;
    let rows=sqlx::query_as(&format!("select {INVITATION_COLUMNS} from app.workspace_invitations where workspace_id=app.current_workspace_id() order by created_at desc,id desc limit 200"))
        .fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(rows)
}

/// Creates a bounded invitation and its replay receipt atomically.
/// # Errors
/// Returns invalid input, insufficient rights, a conflicting command or a database error.
pub async fn create(
    state: &AppState,
    input: CreateInvitation,
    lease: Option<&IdempotencyLease>,
) -> AppResult<Invitation> {
    owner(state)?;
    validate_invitation(&input)?;
    let request_hash = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&input)
                .map_err(|_| AppError::Internal("Cannot encode invitation command".into()))?
        )
    );
    let mut tx = state.begin_request().await?;
    sqlx::query("select id from app.workspaces where id=app.current_workspace_id() for update")
        .execute(&mut *tx)
        .await?;
    let existing: Option<(String, Uuid)> = sqlx::query_as(
        "select request_hash,created_by_actor_id from app.workspace_invitations where public_id=$1",
    )
    .bind(input.public_id)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some((fingerprint, actor)) = existing {
        if fingerprint != request_hash || actor != state.actor_id {
            return Err(AppError::Conflict(
                "Cette invitation appartient à une autre demande.".into(),
            ));
        }
    } else {
        let count:i64=sqlx::query_scalar("select count(*) from app.workspace_invitations where workspace_id=app.current_workspace_id() and status='pending' and expires_at>now()")
            .fetch_one(&mut *tx).await?;
        if count >= 100 {
            return Err(AppError::Invalid(
                "Révoquez des invitations en attente avant d’en créer de nouvelles.".into(),
            ));
        }
        sqlx::query("insert into app.workspace_invitations(public_id,workspace_id,created_by_actor_id,role,label,token_hash,request_hash,expires_at)
            values($1,app.current_workspace_id(),app.current_actor_id(),$2,$3,$4,$5,now()+make_interval(days=>$6))")
            .bind(input.public_id).bind(&input.role).bind(&input.label).bind(&input.token_hash).bind(request_hash)
            .bind(i32::try_from(input.expires_in_days).map_err(|_|AppError::Invalid("Durée invalide".into()))?)
            .execute(&mut *tx).await.map_err(error)?;
        audit(
            &mut tx,
            state,
            "team.invitation.created",
            input.public_id,
            json!({"role":input.role,"expires_in_days":input.expires_in_days}),
        )
        .await?;
    }
    let output = sqlx::query_as(&format!(
        "select {INVITATION_COLUMNS} from app.workspace_invitations where public_id=$1"
    ))
    .bind(input.public_id)
    .fetch_one(&mut *tx)
    .await?;
    crate::company::complete(&mut tx, lease, &output).await?;
    tx.commit().await?;
    Ok(output)
}

/// Revokes an unused invitation. Membership removal remains a separate action.
/// # Errors
/// Returns insufficient rights, a used invitation or a database error.
pub async fn revoke(
    state: &AppState,
    id: Uuid,
    lease: Option<&IdempotencyLease>,
) -> AppResult<Invitation> {
    owner(state)?;
    let mut tx = state.begin_request().await?;
    sqlx::query("select id from app.workspaces where id=app.current_workspace_id() for update")
        .execute(&mut *tx)
        .await?;
    let status: Option<String> = sqlx::query_scalar(
        "select status from app.workspace_invitations where public_id=$1 for update",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    match status.as_deref() {
        None => return Err(AppError::NotFound),
        Some("accepted") => {
            return Err(AppError::Conflict(
                "Cette invitation a été utilisée. Gérez l’accès depuis l’équipe.".into(),
            ));
        }
        Some("pending") => {
            sqlx::query("update app.workspace_invitations set status='revoked',revoked_at=now() where public_id=$1")
                .bind(id).execute(&mut *tx).await?;
            audit(&mut tx, state, "team.invitation.revoked", id, json!({})).await?;
        }
        _ => {}
    }
    let output = sqlx::query_as(&format!(
        "select {INVITATION_COLUMNS} from app.workspace_invitations where public_id=$1"
    ))
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    crate::company::complete(&mut tx, lease, &output).await?;
    tx.commit().await?;
    Ok(output)
}

/// Returns minimal invitation details only on possession of its original token.
/// # Errors
/// Expired, revoked and invalid links all return unavailable.
pub async fn preview(
    state: &AppState,
    id: Uuid,
    input: PreviewInvitation,
) -> AppResult<InvitationPreview> {
    let digest = token_hash(&input.token)?;
    sqlx::query_as("select * from app.preview_workspace_invitation($1,$2)")
        .bind(id)
        .bind(digest)
        .fetch_optional(&state.pool)
        .await
        .map_err(error)?
        .ok_or(AppError::NotFound)
}

/// Joins the company as the verified actor; no workspace selection is trusted.
/// # Errors
/// Returns unavailable for invalid links or withdrawn membership on replay.
pub async fn accept(
    state: &AppState,
    id: Uuid,
    input: AcceptInvitation,
) -> AppResult<WorkspaceSummary> {
    valid_name(&input.display_name)?;
    let digest = token_hash(&input.token)?;
    if state.actor_id.is_nil() {
        return Err(AppError::Unauthorized);
    }
    let mut tx = state.pool.begin().await?;
    sqlx::query("select set_config('app.current_actor_id',$1,true),set_config('app.current_workspace_id','',true),set_config('app.current_workspace_role','',true)")
        .bind(state.actor_id.to_string()).execute(&mut *tx).await?;
    let result = sqlx::query_as("select * from app.accept_workspace_invitation($1,$2,$3)")
        .bind(id)
        .bind(digest)
        .bind(input.display_name.trim())
        .fetch_one(&mut *tx)
        .await
        .map_err(error)?;
    tx.commit().await?;
    Ok(result)
}

async fn audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    state: &AppState,
    action: &str,
    id: Uuid,
    details: serde_json::Value,
) -> AppResult<()> {
    sqlx::query("insert into app.audit_events(workspace_id,actor_id,action,object_kind,object_public_id,after_state)
        values(app.current_workspace_id(),$1,$2,'workspace_invitation',$3,$4)")
        .bind(state.actor_id).bind(action).bind(id).bind(details).execute(&mut **tx).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tokens_are_validated_before_digest_and_never_echoed_in_failures() {
        let secret = "ab".repeat(32);
        let hash = token_hash(&secret).expect("valid token");
        assert_eq!(hash.len(), 64);
        assert_ne!(hash, secret);
        assert_ne!(
            token_hash(&hash).expect("hex digest is not a bearer token"),
            hash
        );
        for token in ["short".to_owned(), "XX".repeat(32), "a".repeat(65)] {
            let failure = token_hash(&token).expect_err("invalid token").to_string();
            assert!(!failure.contains(&token));
        }
    }
    #[test]
    fn invitations_cannot_grant_owner_or_unbounded_lifetime() {
        let mut input = CreateInvitation {
            public_id: Uuid::new_v4(),
            token_hash: "a".repeat(64),
            role: "editor".into(),
            label: "[FICTIF] Équipe".into(),
            expires_in_days: 7,
        };
        assert!(validate_invitation(&input).is_ok());
        input.role = "owner".into();
        assert!(validate_invitation(&input).is_err());
        input.role = "viewer".into();
        input.expires_in_days = 8;
        assert!(validate_invitation(&input).is_err());
        assert!(valid_name("[FICTIF] Camille").is_ok());
        assert!(valid_name(" ").is_err());
    }
}
