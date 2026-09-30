//! Invited-company acceptance with synthetic identities and a disposable database.
use ai_center_server::{
    agent::DeterministicEngine,
    auth::{AuthRuntime, RequestContext},
    company::{
        self,
        models::{CreateCompany, UpdateMember},
    },
    config::AuthMode,
    error::AppError,
    routes,
    service::AppState,
    team::{self, AcceptInvitation, CreateInvitation, PreviewInvitation, SetProfile},
};
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use uuid::Uuid;

async fn isolated_actor() -> Result<AppState> {
    let raw = std::env::var("DATABASE_URL")
        .context("company_context requires the guarded isolated integration stack")?;
    let url = url::Url::parse(&raw).map_err(|_| anyhow::anyhow!("invalid integration URL"))?;
    ensure!(
        url.scheme() == "postgresql"
            && url.host_str() == Some("127.0.0.1")
            && url.port() == Some(55322)
            && url.username() == "ai_center_runtime"
            && url.path() == "/postgres"
            && url.query().is_none()
            && url.fragment().is_none(),
        "isolated runtime database on 55322 required"
    );
    ensure!(
        std::env::var("AI_CENTER_EXPECT_DATABASE_ROLE").as_deref() == Ok("ai_center_runtime")
            && std::env::var("AI_CENTER_AGENT_MODE").as_deref() == Ok("deterministic"),
        "explicit safe integration mode required"
    );
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&raw)
        .await?;
    let (role, superuser, bypass): (String, bool, bool) = sqlx::query_as(
        "select current_user::text,rolsuper,rolbypassrls from pg_roles where rolname=current_user",
    )
    .fetch_one(&pool)
    .await?;
    ensure!(
        role == "ai_center_runtime" && !superuser && !bypass,
        "runtime RLS must be enforced"
    );
    Ok(AppState {
        pool,
        engine: Arc::new(DeterministicEngine),
        providers: None,
        workspace_id: Uuid::nil(),
        workspace_internal_id: None,
        workspace_role: "viewer".into(),
        actor_id: Uuid::new_v4(),
        agent_mode: "deterministic",
        steward_trigger: None,
    })
}

async fn select(state: &AppState, workspace: Uuid, actor: Uuid) -> Result<AppState> {
    let (internal, role): (i64, String) =
        sqlx::query_as("select workspace_id,role from app.authorize_workspace_member($1,$2)")
            .bind(workspace)
            .bind(actor)
            .fetch_one(&state.pool)
            .await?;
    Ok(state.scoped(&RequestContext {
        actor_id: actor,
        workspace_id: workspace,
        workspace_internal_id: Some(internal),
        workspace_role: role,
    }))
}

async fn create(state: &AppState, id: Uuid) -> Result<AppState> {
    let overview = company::create(
        state,
        CreateCompany {
            public_id: id,
            name: "[FICTIF] Société de test".into(),
            description: "Contexte partagé de test".into(),
        },
    )
    .await?;
    assert!(overview.setup_complete);
    assert_eq!(overview.agents.len(), 5);
    assert!(
        overview.projects.is_empty(),
        "company scope must not become a customer project"
    );
    select(state, id, state.actor_id).await
}

fn invitation(id: Uuid, token: &str, role: &str) -> CreateInvitation {
    CreateInvitation {
        public_id: id,
        token_hash: format!("{:x}", Sha256::digest(token.as_bytes())),
        role: role.into(),
        label: "[FICTIF] Équipe".into(),
        expires_in_days: 7,
    }
}
fn acceptance(token: &str) -> AcceptInvitation {
    AcceptInvitation {
        token: token.into(),
        display_name: "[FICTIF] Alex".into(),
    }
}
fn preview(token: &str) -> PreviewInvitation {
    PreviewInvitation {
        token: token.into(),
    }
}

#[tokio::test]
async fn invitation_is_single_use_and_revoked_membership_never_revives_on_replay() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let id = Uuid::new_v4();
    let token = "a1".repeat(32);
    let created = team::create(&owner, invitation(id, &token, "viewer"), None).await?;
    let encoded = serde_json::to_string(&created)?;
    assert!(!encoded.contains(&token));
    assert!(!encoded.contains("token_hash"));
    assert_eq!(
        team::create(&owner, invitation(id, &token, "viewer"), None)
            .await?
            .public_id,
        id
    );
    assert!(matches!(
        team::create(&owner, invitation(id, &token, "editor"), None).await,
        Err(AppError::Conflict(_))
    ));
    assert_eq!(
        team::preview(&actor, id, preview(&token))
            .await?
            .workspace_public_id,
        owner.workspace_id
    );
    let digest = format!("{:x}", Sha256::digest(token.as_bytes()));
    assert!(matches!(
        team::preview(&actor, id, preview(&digest)).await,
        Err(AppError::NotFound)
    ));
    let mut invitee = actor.clone();
    invitee.actor_id = Uuid::new_v4();
    let accepted = team::accept(&invitee, id, acceptance(&token)).await?;
    assert_eq!(accepted.role, "viewer");
    assert_eq!(
        team::accept(&invitee, id, acceptance(&token))
            .await?
            .public_id,
        owner.workspace_id
    );
    let member = select(&actor, owner.workspace_id, invitee.actor_id).await?;
    assert!(matches!(
        team::create(&member, invitation(Uuid::new_v4(), &token, "editor"), None).await,
        Err(AppError::Forbidden)
    ));
    assert!(matches!(
        team::invitations(&member).await,
        Err(AppError::Forbidden)
    ));
    let profile = team::profile(
        &member,
        SetProfile {
            display_name: "[FICTIF] Lecteur".into(),
        },
    )
    .await?;
    assert_eq!(profile.display_name, "[FICTIF] Lecteur");
    let mut other = actor.clone();
    other.actor_id = Uuid::new_v4();
    assert!(matches!(
        team::accept(&other, id, acceptance(&token)).await,
        Err(AppError::NotFound)
    ));
    company::update_member(
        &owner,
        profile.public_id,
        UpdateMember {
            role: None,
            invitation_status: Some("revoked".into()),
        },
        None,
    )
    .await?;
    assert!(matches!(
        team::accept(&invitee, id, acceptance(&token)).await,
        Err(AppError::NotFound)
    ));
    assert!(
        team::profile(
            &member,
            SetProfile {
                display_name: "Unauthorized".into()
            }
        )
        .await
        .is_err()
    );
    let replacement = Uuid::new_v4();
    team::create(&owner, invitation(replacement, &token, "editor"), None).await?;
    assert_eq!(
        team::accept(&invitee, replacement, acceptance(&token))
            .await?
            .role,
        "editor"
    );
    owner.pool.close().await;
    Ok(())
}

#[tokio::test]
async fn old_unused_invitation_cannot_bypass_later_member_removal() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let links = [
        (Uuid::new_v4(), "d4".repeat(32)),
        (Uuid::new_v4(), "e5".repeat(32)),
    ];
    for (id, token) in &links {
        team::create(&owner, invitation(*id, token, "editor"), None).await?;
    }
    let mut invitee = actor.clone();
    invitee.actor_id = Uuid::new_v4();
    team::accept(&invitee, links[0].0, acceptance(&links[0].1)).await?;
    let member = team::members(&owner)
        .await?
        .into_iter()
        .find(|member| member.actor_id == invitee.actor_id)
        .context("accepted invitee")?;
    company::update_member(
        &owner,
        member.public_id,
        UpdateMember {
            role: None,
            invitation_status: Some("revoked".into()),
        },
        None,
    )
    .await?;
    for (id, token) in &links {
        assert!(matches!(
            team::accept(&invitee, *id, acceptance(token)).await,
            Err(AppError::NotFound)
        ));
    }
    let access: Option<i64> =
        sqlx::query_scalar("select workspace_id from app.authorize_workspace_member($1,$2)")
            .bind(owner.workspace_id)
            .bind(invitee.actor_id)
            .fetch_optional(&owner.pool)
            .await?;
    assert!(
        access.is_none(),
        "neither old link may restore withdrawn access"
    );
    let replacement = Uuid::new_v4();
    let new_token = "f6".repeat(32);
    team::create(&owner, invitation(replacement, &new_token, "viewer"), None).await?;
    assert_eq!(
        team::accept(&invitee, replacement, acceptance(&new_token))
            .await?
            .role,
        "viewer"
    );
    owner.pool.close().await;
    Ok(())
}

#[tokio::test]
async fn concurrent_acceptance_and_company_boundaries_remain_atomic() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let foreign = create(&actor, Uuid::new_v4()).await?;
    let id = Uuid::new_v4();
    let token = "b2".repeat(32);
    team::create(&owner, invitation(id, &token, "editor"), None).await?;
    assert!(team::invitations(&foreign).await?.is_empty());
    assert!(matches!(
        team::revoke(&foreign, id, None).await,
        Err(AppError::NotFound)
    ));
    let mut first = actor.clone();
    first.actor_id = Uuid::new_v4();
    let mut second = actor.clone();
    second.actor_id = Uuid::new_v4();
    let (a, b) = tokio::join!(
        team::accept(&first, id, acceptance(&token)),
        team::accept(&second, id, acceptance(&token))
    );
    assert_ne!(
        a.is_ok(),
        b.is_ok(),
        "exactly one identity can consume the link"
    );
    assert_eq!(team::members(&owner).await?.len(), 2);
    let revoked = Uuid::new_v4();
    team::create(&owner, invitation(revoked, &token, "viewer"), None).await?;
    team::revoke(&owner, revoked, None).await?;
    assert!(matches!(
        team::preview(&actor, revoked, preview(&token)).await,
        Err(AppError::NotFound)
    ));
    assert!(matches!(
        team::accept(&first, revoked, acceptance(&token)).await,
        Err(AppError::NotFound)
    ));
    let members = team::members(&owner).await?;
    let successor = members
        .iter()
        .find(|member| member.actor_id != owner.actor_id)
        .context("successor")?;
    company::update_member(
        &owner,
        successor.public_id,
        UpdateMember {
            role: Some("owner".into()),
            invitation_status: None,
        },
        None,
    )
    .await?;
    let withdrawn = Uuid::new_v4();
    team::create(&owner, invitation(withdrawn, &token, "viewer"), None).await?;
    let self_member = members
        .iter()
        .find(|member| member.actor_id == owner.actor_id)
        .context("creator")?;
    let successor_scope = select(&actor, owner.workspace_id, successor.actor_id).await?;
    company::update_member(
        &successor_scope,
        self_member.public_id,
        UpdateMember {
            role: Some("editor".into()),
            invitation_status: None,
        },
        None,
    )
    .await?;
    assert!(matches!(
        team::preview(&actor, withdrawn, preview(&token)).await,
        Err(AppError::NotFound)
    ));
    assert!(matches!(
        team::accept(&second, withdrawn, acceptance(&token)).await,
        Err(AppError::NotFound)
    ));
    owner.pool.close().await;
    Ok(())
}

#[tokio::test]
async fn public_preview_does_not_bypass_identity_for_acceptance_or_other_routes() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let id = Uuid::new_v4();
    let token = "c3".repeat(32);
    team::create(&owner, invitation(id, &token, "viewer"), None).await?;
    let auth = Arc::new(AuthRuntime::new(
        AuthMode::Supabase,
        Uuid::nil(),
        Uuid::nil(),
        Some("http://127.0.0.1:55321".into()),
    )?);
    let app = routes::router(
        Arc::new(actor.clone()),
        auth,
        None,
        vec!["http://127.0.0.1".parse()?],
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let base = format!("http://{}", listener.local_addr()?);
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{base}/api/team/invitations/{id}/preview"))
        .json(&serde_json::json!({"token":token}))
        .send()
        .await?;
    assert_eq!(response.status(), 200);
    assert_eq!(
        response
            .headers()
            .get("cache-control")
            .context("private response")?,
        "no-store"
    );
    let body = response.text().await?;
    assert!(!body.contains(&token));
    assert!(!body.contains("token_hash"));
    for path in [
        format!("/api/team/invitations/{id}/accept"),
        format!("/api/team/invitations/{id}/revoke"),
        "/api/team/invitations".into(),
    ] {
        let response = client
            .post(format!("{base}{path}"))
            .json(&serde_json::json!({"token":token,"display_name":"[FICTIF] Alex"}))
            .send()
            .await?;
        assert_eq!(response.status(), 401, "{path}");
    }
    handle.abort();
    owner.pool.close().await;
    Ok(())
}
