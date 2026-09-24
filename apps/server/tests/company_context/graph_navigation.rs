use super::*;
use ai_center_server::models::{CreateHandoff, ReviseKnowledge};

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn graph_navigation_preserves_exact_sources_and_scope_boundaries() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let foreign = create(&actor, Uuid::new_v4()).await?;
    let (project, product) = ready_project(&owner, "[FICTIF] Navigation").await?;
    let (neighbor, neighbor_session) = ready_project(&owner, "[FICTIF] Voisin").await?;
    let (entry, version) = fixture_knowledge(
        &owner,
        project,
        "constraint",
        "[FICTIF] Texte historique exact.",
    )
    .await?;
    let pack = compile_for(&owner, project, product, "tech").await?;
    let handoff = service::create_handoff(
        &owner,
        project,
        CreateHandoff {
            source_session_id: product,
            context_pack_id: pack.public_id,
        },
    )
    .await?;
    let mut tx = owner.pool.begin().await?;
    sqlx::query("select set_config('app.current_actor_id',$1,true),set_config('app.current_workspace_id',$2,true),set_config('app.current_workspace_role','owner',true)")
        .bind(owner.actor_id.to_string()).bind(owner.workspace_internal_id.context("scope")?.to_string()).execute(&mut *tx).await?;
    let task: Uuid = sqlx::query_scalar("insert into app.tasks(workspace_id,project_id,context_pack_id,contract_id,title)
        select c.workspace_id,c.project_id,c.id,d.id,'[FICTIF] Travail lié' from app.context_packs c
        join app.projects p on p.id=c.project_id join app.deliverable_contracts d on d.template_id=p.template_id and d.contract_key=c.task_kind
        where c.public_id=$1 returning public_id")
        .bind(pack.public_id).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    let graph = company::graph(
        &owner,
        GraphQuery {
            project_id: Some(project),
            limit: None,
        },
    )
    .await?;
    for (kind, id) in [
        ("session", product),
        ("session", handoff.target_session_public_id),
        ("task", task),
    ] {
        let node = graph
            .nodes
            .iter()
            .find(|node| node.id == id && node.kind == kind)
            .context("navigable node")?;
        assert!(
            node.app_path
                .as_deref()
                .is_some_and(|path| path.contains(&id.to_string()))
        );
    }
    assert!(!graph.nodes.iter().any(|node| node.id == neighbor_session));
    for id in [task, handoff.target_session_public_id] {
        assert!(
            graph
                .edges
                .iter()
                .any(|edge| edge.source_public_id == id && edge.target_public_id == pack.public_id)
        );
    }
    let source =
        company::graph_source::read(&owner, project, "context_pack", pack.public_id).await?;
    assert_eq!(source["content"], pack.content);
    assert!(
        source["links"]
            .as_array()
            .context("links")?
            .iter()
            .any(|link| link["app_path"]
                .as_str()
                .is_some_and(|path| path.ends_with(&version.to_string())))
    );
    let task_source = company::graph_source::read(&owner, project, "task", task).await?;
    assert!(
        task_source["links"][0]["app_path"]
            .as_str()
            .context("pack link")?
            .ends_with(&pack.public_id.to_string())
    );
    service::revise_knowledge_for_project(
        &owner,
        project,
        entry,
        ReviseKnowledge {
            title: None,
            statement: "[FICTIF] Texte révisé distinct.".into(),
            rationale: None,
        },
    )
    .await?;
    let old = company::graph_source::read(&owner, project, "knowledge", version).await?;
    assert_eq!(old["status"], "superseded");
    assert_eq!(old["version_number"], 1);
    assert_eq!(
        old["content"]["statement"],
        "[FICTIF] Texte historique exact."
    );
    assert_eq!(
        company::graph_source::read(&owner, project, "context_pack", pack.public_id).await?["status"],
        "stale"
    );
    for (state, scope, kind, id) in [
        (&foreign, project, "knowledge", version),
        (&owner, neighbor, "knowledge", version),
        (&owner, project, "provider_connections", version),
        (&owner, neighbor, "context_pack", pack.public_id),
        (&foreign, project, "task", task),
    ] {
        assert!(matches!(
            company::graph_source::read(state, scope, kind, id).await,
            Err(AppError::NotFound)
        ));
    }
    let viewer_id = Uuid::new_v4();
    add_member(&owner, viewer_id, "viewer").await?;
    let viewer = select(&owner, owner.workspace_id, viewer_id).await?;
    assert_eq!(
        company::graph_source::read(&viewer, project, "knowledge", version).await?,
        old
    );
    let edge_id = Uuid::new_v4();
    let edge = || CreateGraphEdge {
        public_id: edge_id,
        source: GraphEndpoint {
            kind: "session".into(),
            public_id: product,
            project_public_id: project,
        },
        target: GraphEndpoint {
            kind: "task".into(),
            public_id: task,
            project_public_id: project,
        },
        edge_type: "informs".into(),
    };
    assert!(matches!(
        company::create_edge(&viewer, edge(), None).await,
        Err(AppError::Forbidden)
    ));
    company::create_edge(&owner, edge(), None).await?;
    let mut forged = edge();
    forged.public_id = Uuid::new_v4();
    forged.source.project_public_id = neighbor;
    assert!(matches!(
        company::create_edge(&owner, forged, None).await,
        Err(AppError::NotFound)
    ));
    let graph = company::graph(
        &owner,
        GraphQuery {
            project_id: Some(project),
            limit: None,
        },
    )
    .await?;
    assert!(graph.edges.iter().any(|item| item.id == edge_id
        && item.source_kind == "session"
        && item.target_kind == "task"));
    actor.pool.close().await;
    Ok(())
}
