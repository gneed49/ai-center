use super::*;
use ai_center_server::company::data::{self, ArchiveProject, RenameProject};

#[tokio::test]
async fn rename_requires_editor_and_exact_revision_without_losing_project_history() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let (project, session) = ready_project(&owner, "[FICTIF] Nom initial").await?;
    let initial = service::list_projects(&owner).await?.remove(0);
    let editor_id = Uuid::new_v4();
    add_member(&owner, editor_id, "editor").await?;
    let editor = select(&owner, owner.workspace_id, editor_id).await?;
    let input = |name: &str| RenameProject {
        name: name.into(),
        expected_updated_at: initial.updated_at,
    };
    assert!(matches!(
        data::rename(&editor, project, input(" \n "), None).await,
        Err(AppError::Invalid(_))
    ));
    let (first, second) = tokio::join!(
        data::rename(&editor, project, input("[FICTIF] Nom A"), None),
        data::rename(&owner, project, input("[FICTIF] Nom B"), None)
    );
    let updated = match (first, second) {
        (Ok(updated), Err(AppError::Conflict(_))) | (Err(AppError::Conflict(_)), Ok(updated)) => {
            updated
        }
        results => anyhow::bail!("exactly one concurrent edit must succeed: {results:?}"),
    };
    assert_eq!(updated.public_id, project);
    assert_eq!(updated.graph_version, initial.graph_version + 1);
    assert_eq!(updated.objective, initial.objective);
    assert!(service::session_view(&editor, session).await.is_ok());
    let viewer_id = Uuid::new_v4();
    add_member(&owner, viewer_id, "viewer").await?;
    let viewer = select(&owner, owner.workspace_id, viewer_id).await?;
    assert!(matches!(
        data::rename(&viewer, project, input("[FICTIF] Refusé"), None).await,
        Err(AppError::Forbidden)
    ));
    let foreign = create(&actor, Uuid::new_v4()).await?;
    assert!(matches!(
        data::rename(&foreign, project, input("[FICTIF] Étranger"), None).await,
        Err(AppError::NotFound)
    ));
    let company = company::overview(&owner)
        .await?
        .company_scope
        .context("company scope")?;
    assert!(matches!(
        data::rename(
            &owner,
            company.project_public_id,
            input("[FICTIF] Société"),
            None
        )
        .await,
        Err(AppError::Invalid(_))
    ));
    let archived = data::archive(&owner, project, ArchiveProject { archived: true }, None).await?;
    assert!(matches!(
        data::rename(
            &owner,
            project,
            RenameProject {
                name: "[FICTIF] Archive".into(),
                expected_updated_at: archived.updated_at
            },
            None
        )
        .await,
        Err(AppError::Conflict(_))
    ));
    Ok(())
}

#[tokio::test]
async fn explicit_archived_graph_keeps_history_outside_the_active_federation() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let (project, _) = ready_project(&owner, "[FICTIF] Graphe archivé").await?;
    data::archive(&owner, project, ArchiveProject { archived: true }, None).await?;
    let viewer_id = Uuid::new_v4();
    add_member(&owner, viewer_id, "viewer").await?;
    let viewer = select(&owner, owner.workspace_id, viewer_id).await?;
    let history = company::graph(
        &viewer,
        GraphQuery {
            project_id: Some(project),
            limit: None,
        },
    )
    .await?;
    assert!(
        history
            .nodes
            .iter()
            .any(|node| node.id == project && node.kind == "scope" && node.status == "archived")
    );
    assert!(
        history
            .nodes
            .iter()
            .any(|node| node.project_public_id == project && node.kind == "knowledge")
    );
    assert!(
        history
            .source_graph_versions
            .iter()
            .any(|scope| scope.project_public_id == project)
    );
    let active = company::graph(
        &viewer,
        GraphQuery {
            project_id: None,
            limit: None,
        },
    )
    .await?;
    assert!(
        active
            .nodes
            .iter()
            .all(|node| node.project_public_id != project)
    );
    assert!(
        active
            .source_graph_versions
            .iter()
            .all(|scope| scope.project_public_id != project)
    );
    Ok(())
}
