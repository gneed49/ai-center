use super::*;
use ai_center_server::{
    company::knowledge_library::{self, Query},
    models::ReviseKnowledge,
};

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn library_pages_all_knowledge_and_preserves_history_without_tenant_leakage() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let foreign = create(&actor, Uuid::new_v4()).await?;
    let project = service::create_project(
        &owner,
        CreateProject {
            name: "[FICTIF] Grande mémoire".into(),
            objective: "Retrouver les anciennes sources.".into(),
        },
    )
    .await?;
    let mut versions = Vec::new();
    for index in 0..31 {
        versions.push(
            fixture_knowledge(
                &owner,
                project.public_id,
                "business_rule",
                &format!("[FICTIF] Règle de bibliothèque {index:02}."),
            )
            .await?,
        );
    }
    let filter = || Query {
        project_id: Some(project.public_id),
        ..Query::default()
    };
    let first = knowledge_library::list(&owner, filter()).await?;
    assert_eq!(first["total"], 31);
    assert_eq!(first["items"].as_array().context("page")?.len(), 25);
    let second = knowledge_library::list(
        &owner,
        Query {
            offset: Some(25),
            ..filter()
        },
    )
    .await?;
    assert_eq!(second["items"].as_array().context("second page")?.len(), 6);
    assert!(
        second["items"]
            .as_array()
            .context("old source")?
            .iter()
            .any(|item| item["public_id"] == versions[0].1.to_string())
    );
    let searched = knowledge_library::list(
        &owner,
        Query {
            q: Some("bibliothèque 00".into()),
            ..filter()
        },
    )
    .await?;
    assert_eq!(searched["total"], 1);
    assert_eq!(searched["items"][0]["public_id"], versions[0].1.to_string());
    service::revise_knowledge_for_project(
        &owner,
        project.public_id,
        versions[0].0,
        ReviseKnowledge {
            title: None,
            statement: "[FICTIF] Règle révisée.".into(),
            rationale: None,
        },
    )
    .await?;
    assert_eq!(
        knowledge_library::list(
            &owner,
            Query {
                q: Some("bibliothèque 00".into()),
                ..filter()
            }
        )
        .await?["total"],
        0
    );
    let history = knowledge_library::list(
        &owner,
        Query {
            q: Some("bibliothèque 00".into()),
            history: Some(true),
            ..filter()
        },
    )
    .await?;
    assert_eq!(history["total"], 1);
    assert_eq!(history["items"][0]["status"], "superseded");
    assert_eq!(history["items"][0]["version_number"], 1);
    assert_eq!(
        knowledge_library::list(
            &owner,
            Query {
                q: Some("%".into()),
                ..filter()
            }
        )
        .await?["total"],
        0
    );
    assert_eq!(
        knowledge_library::list(&foreign, Query::default()).await?["total"],
        0
    );
    assert!(matches!(
        knowledge_library::list(&foreign, filter()).await,
        Err(AppError::NotFound)
    ));
    let viewer_id = Uuid::new_v4();
    add_member(&owner, viewer_id, "viewer").await?;
    let viewer = select(&owner, owner.workspace_id, viewer_id).await?;
    assert_eq!(
        knowledge_library::list(
            &viewer,
            Query {
                history: Some(true),
                ..filter()
            }
        )
        .await?["total"],
        32
    );
    company::data::archive(
        &owner,
        project.public_id,
        company::data::ArchiveProject { archived: true },
        None,
    )
    .await?;
    let archived = knowledge_library::list(&viewer, filter()).await?;
    assert_eq!(archived["total"], 31);
    assert_eq!(archived["items"][0]["project_status"], "archived");
    for query in [
        Query {
            limit: Some(0),
            ..filter()
        },
        Query {
            limit: Some(101),
            ..filter()
        },
        Query {
            offset: Some(1_000_001),
            ..filter()
        },
        Query {
            q: Some("a".repeat(201)),
            ..filter()
        },
    ] {
        assert!(matches!(
            knowledge_library::list(&owner, query).await,
            Err(AppError::Invalid(_))
        ));
    }
    actor.pool.close().await;
    Ok(())
}
