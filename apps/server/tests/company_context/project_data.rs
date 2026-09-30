use super::*;
use ai_center_server::{
    artifacts::{self, CreateArtifact},
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

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn erasure_waits_for_steward_and_removes_only_project_receipts() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let foreign = create(&actor, Uuid::new_v4()).await?;
    let admin = erasure::isolated_admin().await?;
    let workspace = owner.workspace_internal_id.context("scope")?;
    let target = bare_project(&owner, "[FICTIF] Progression à effacer").await?;
    let artifact = artifacts::create(
        &owner,
        target,
        CreateArtifact {
            artifact_type: "specification".into(),
            title: "[FICTIF] Source examinée".into(),
            body_markdown: "[FICTIF] Source interne indépendante.".into(),
            structured_content: json!({}),
            sources: vec![],
        },
        None,
    )
    .await?;
    sqlx::query("insert into app.steward_scan_sources(workspace_id,project_id,source_public_id,source_kind,examined_pairs,omitted_neighbors) select $1,id,$3,'artifact_document_version',2,3 from app.projects where public_id=$2")
        .bind(workspace).bind(target).bind(artifact.current_version.public_id).execute(&admin).await?;
    sqlx::query("insert into app.steward_scan_progress(workspace_id,status,lease_token,lease_until) values($1,'running',$2,now()+interval '2 minutes')")
        .bind(workspace).bind(Uuid::new_v4()).execute(&admin).await?;
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
    let foreign_before =
        erasure::tenant_hash(&admin, foreign.workspace_internal_id.context("scope")?).await?;
    let project_manifest = preview(&admin, owner.workspace_id, target).await?;
    assert_eq!(project_manifest["eligible"], true);
    assert_eq!(project_manifest["row_counts"]["steward_scan_sources"], 1);
    assert_eq!(project_manifest["row_counts"]["steward_scan_progress"], 0);
    let company_manifest: Value = sqlx::query_scalar("select app.operator_workspace_manifest($1)")
        .bind(owner.workspace_id)
        .fetch_one(&admin)
        .await?;
    for (query, project_id, receipt) in [
        (
            "select app.operator_purge_project($1,$2,$2,$3)",
            target,
            project_manifest["receipt"].as_str(),
        ),
        (
            "select app.operator_purge_workspace($1,$2,$3)",
            owner.workspace_id,
            company_manifest["receipt"].as_str(),
        ),
    ] {
        let failure = sqlx::query(query)
            .bind(owner.workspace_id)
            .bind(project_id)
            .bind(receipt)
            .execute(&admin)
            .await
            .expect_err("active company steward must block erasure");
        assert!(
            failure.to_string().contains("active work"),
            "unexpected rejection: {failure}"
        );
    }
    sqlx::query("update app.steward_scan_progress set status='idle',lease_token=null,lease_until=null where workspace_id=$1")
        .bind(workspace).execute(&admin).await?;
    let progress_before: Value = sqlx::query_scalar(
        "select to_jsonb(t) from app.steward_scan_progress t where workspace_id=$1",
    )
    .bind(workspace)
    .fetch_one(&admin)
    .await?;
    let refreshed = preview(&admin, owner.workspace_id, target).await?;
    let erased: Value = sqlx::query_scalar("select app.operator_purge_project($1,$2,$2,$3)")
        .bind(owner.workspace_id)
        .bind(target)
        .bind(refreshed["receipt"].as_str())
        .fetch_one(&admin)
        .await?;
    assert_eq!(erased["deleted_rows"]["steward_scan_sources"], 1);
    assert_eq!(erased["retained_application_data_verified"], true);
    let progress_after: Value = sqlx::query_scalar(
        "select to_jsonb(t) from app.steward_scan_progress t where workspace_id=$1",
    )
    .bind(workspace)
    .fetch_one(&admin)
    .await?;
    assert_eq!(progress_after, progress_before);
    let company_manifest: Value = sqlx::query_scalar("select app.operator_workspace_manifest($1)")
        .bind(owner.workspace_id)
        .fetch_one(&admin)
        .await?;
    let company_erased: Value = sqlx::query_scalar("select app.operator_purge_workspace($1,$1,$2)")
        .bind(owner.workspace_id)
        .bind(company_manifest["receipt"].as_str())
        .fetch_one(&admin)
        .await?;
    assert_eq!(company_erased["deleted_rows"]["steward_scan_progress"], 1);
    assert_eq!(
        erasure::tenant_hash(&admin, foreign.workspace_internal_id.context("scope")?).await?,
        foreign_before
    );
    Ok(())
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
