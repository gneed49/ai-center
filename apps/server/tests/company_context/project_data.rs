use super::*;
use ai_center_server::{
    automation::{self, SetControl},
    company::{
        data::{self, ArchiveProject},
        project_data,
    },
};
use serde_json::Value;

async fn bare_project(owner: &AppState, name: &str) -> Result<Uuid> {
    Ok(service::create_project(
        owner,
        CreateProject {
            name: name.into(),
            objective: "[FICTIF] Portabilité".into(),
        },
    )
    .await?
    .public_id)
}
async fn preview(admin: &sqlx::PgPool, workspace: Uuid, project: Uuid) -> Result<Value> {
    Ok(
        sqlx::query_scalar("select app.operator_project_manifest($1,$2)")
            .bind(workspace)
            .bind(project)
            .fetch_one(admin)
            .await?,
    )
}

#[tokio::test]
async fn export_keeps_exact_direct_sources_without_neighbor_conversations() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let (project, session) = ready_project(&owner, "[FICTIF] Export cible").await?;
    let (neighbor, _) = ready_project(&owner, "[FICTIF] Sources voisines").await?;
    link_projects(&owner, project, neighbor).await?;
    let pack = compile_for(&owner, project, session, "tech").await?;
    let exported = project_data::export(&owner, project).await?;
    assert!(exported.complete);
    assert_eq!(exported.data["projects"].len(), 1);
    assert_eq!(
        exported.data["projects"][0]["public_id"],
        project.to_string()
    );
    assert!(
        exported.data["context_packs"]
            .iter()
            .any(|row| row["public_id"] == pack.public_id.to_string())
    );
    assert!(
        exported
            .referenced_sources
            .iter()
            .any(|row| row["source_project_public_id"] == neighbor.to_string())
    );
    assert!(
        exported
            .referenced_sources
            .iter()
            .all(|row| row["source_kind"] != "session")
    );
    assert!(!exported.data.contains_key("work_tool_connections"));
    assert!(!exported.data.contains_key("workspace_members"));
    let viewer_actor = Uuid::new_v4();
    add_member(&owner, viewer_actor, "viewer").await?;
    let viewer = select(&owner, owner.workspace_id, viewer_actor).await?;
    assert!(matches!(
        project_data::export(&viewer, project).await,
        Err(AppError::Forbidden)
    ));
    let foreign = create(&actor, Uuid::new_v4()).await?;
    assert!(matches!(
        project_data::export(&foreign, project).await,
        Err(AppError::NotFound)
    ));
    let scope = company::overview(&owner)
        .await?
        .company_scope
        .context("scope")?
        .project_public_id;
    assert!(matches!(
        project_data::export(&owner, scope).await,
        Err(AppError::Invalid(_))
    ));
    data::archive(&owner, project, ArchiveProject { archived: true }, None).await?;
    assert_eq!(
        project_data::export(&owner, project).await?.data["projects"][0]["status"],
        "archived"
    );
    Ok(())
}

#[tokio::test]
async fn project_erasure_preserves_other_projects_and_refuses_dependencies() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let foreign = create(&actor, Uuid::new_v4()).await?;
    let admin = erasure::isolated_admin().await?;
    let target = bare_project(&owner, "[FICTIF] Effacement isolé").await?;
    let retained = bare_project(&owner, "[FICTIF] Conservé").await?;
    let retained_export =
        serde_json::to_value(project_data::export(&owner, retained).await?)?["data"].clone();
    let foreign_before =
        erasure::tenant_hash(&admin, foreign.workspace_internal_id.context("scope")?).await?;
    automation::set(
        &owner,
        SetControl {
            enabled: false,
            expected_generation: 0,
        },
        None,
    )
    .await?;
    data::archive(&owner, target, ArchiveProject { archived: true }, None).await?;
    let manifest = preview(&admin, owner.workspace_id, target).await?;
    assert_eq!(manifest["eligible"], true, "{manifest}");
    assert!(
        sqlx::query("select app.operator_project_manifest($1,$2)")
            .bind(owner.workspace_id)
            .bind(target)
            .execute(&owner.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("select app.operator_purge_project($1,$2,$3,$4)")
            .bind(owner.workspace_id)
            .bind(target)
            .bind(retained)
            .bind(manifest["receipt"].as_str())
            .execute(&admin)
            .await
            .is_err()
    );
    let receipt: Value = sqlx::query_scalar("select app.operator_purge_project($1,$2,$2,$3)")
        .bind(owner.workspace_id)
        .bind(target)
        .bind(manifest["receipt"].as_str())
        .fetch_one(&admin)
        .await?;
    assert_eq!(receipt["retained_application_data_verified"], true);
    assert!(matches!(
        project_data::export(&owner, target).await,
        Err(AppError::NotFound)
    ));
    assert_eq!(
        serde_json::to_value(project_data::export(&owner, retained).await?)?["data"],
        retained_export
    );
    assert_eq!(
        erasure::tenant_hash(&admin, foreign.workspace_internal_id.context("scope")?).await?,
        foreign_before
    );
    let linked = bare_project(&owner, "[FICTIF] Avec dépendance").await?;
    link_projects(&owner, linked, retained).await?;
    data::archive(&owner, linked, ArchiveProject { archived: true }, None).await?;
    let blocked = preview(&admin, owner.workspace_id, linked).await?;
    assert_eq!(blocked["eligible"], false);
    assert!(
        sqlx::query("select app.operator_purge_project($1,$2,$2,$3)")
            .bind(owner.workspace_id)
            .bind(linked)
            .bind(blocked["receipt"].as_str())
            .execute(&admin)
            .await
            .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn unexpected_delete_side_effect_rolls_back_project_erasure() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let admin = erasure::isolated_admin().await?;
    let target = bare_project(&owner, "[FICTIF] Rollback").await?;
    let retained = bare_project(&owner, "[FICTIF] Intact").await?;
    automation::set(
        &owner,
        SetControl {
            enabled: false,
            expected_generation: 0,
        },
        None,
    )
    .await?;
    data::archive(&owner, target, ArchiveProject { archived: true }, None).await?;
    let manifest = preview(&admin, owner.workspace_id, target).await?;
    let before =
        erasure::tenant_hash(&admin, owner.workspace_internal_id.context("scope")?).await?;
    let mut tx = admin.begin().await?;
    // A deliberately unexpected trigger is transaction-local and never survives this test.
    sqlx::raw_sql(&format!("create function app.fictitious_project_side_effect() returns trigger language plpgsql as $$ begin update app.projects set name='[FICTIF] Changed by trigger' where public_id='{retained}'; return old; end; $$; create trigger fictitious_project_side_effect after delete on app.projects for each row execute function app.fictitious_project_side_effect();")).execute(&mut *tx).await?;
    let failed = sqlx::query("select app.operator_purge_project($1,$2,$2,$3)")
        .bind(owner.workspace_id)
        .bind(target)
        .bind(manifest["receipt"].as_str())
        .execute(&mut *tx)
        .await;
    assert!(
        failed.is_err(),
        "a side effect outside the deletion set must roll back"
    );
    tx.rollback().await?;
    assert_eq!(
        erasure::tenant_hash(&admin, owner.workspace_internal_id.context("scope")?).await?,
        before
    );
    Ok(())
}
