//! Company tests require the guarded disposable `PostgreSQL` stack. Every name
//! is fictional; no identity provider, email or model provider is contacted.
use ai_center_server::{
    agent::DeterministicEngine,
    auth::{AuthRuntime, RequestContext},
    company::{
        self,
        models::{
            CompanyInput, CreateCompany, CreateGraphEdge, GraphEndpoint, GraphQuery, UpdateMember,
        },
    },
    config::AuthMode,
    error::AppError,
    models::{CreateProject, CreateSession},
    routes,
    service::{self, AppState},
};
use anyhow::{Context, Result, ensure};
use serde_json::json;
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

async fn add_member(owner: &AppState, actor: Uuid, role: &str) -> Result<Uuid> {
    let mut tx = owner.pool.begin().await?;
    sqlx::query("select set_config('app.current_actor_id',$1,true),set_config('app.current_workspace_id',$2,true),set_config('app.current_workspace_role','owner',true)")
        .bind(owner.actor_id.to_string()).bind(owner.workspace_internal_id.context("scope")?.to_string()).execute(&mut *tx).await?;
    let member=sqlx::query_scalar("insert into app.workspace_members(workspace_id,actor_id,role,invitation_status,invited_by_actor_id,accepted_at)
        values(app.current_workspace_id(),$1,$2,'accepted',app.current_actor_id(),now()) returning public_id")
        .bind(actor).bind(role).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(member)
}

#[tokio::test]
async fn bootstrap_without_membership_is_idempotent_scoped_and_bounded() -> Result<()> {
    let actor = isolated_actor().await?;
    let first = Uuid::new_v4();
    let owner = create(&actor, first).await?;
    let replay = create(&actor, first).await?;
    assert_eq!(owner.workspace_internal_id, replay.workspace_internal_id);
    assert!(matches!(
        company::create(
            &actor,
            CreateCompany {
                public_id: first,
                name: "Different request".into(),
                description: String::new()
            }
        )
        .await,
        Err(AppError::Conflict(_))
    ));
    let mut foreign_actor = actor.clone();
    foreign_actor.actor_id = Uuid::new_v4();
    assert!(
        company::create(
            &foreign_actor,
            CreateCompany {
                public_id: first,
                name: "[FICTIF] Société de test".into(),
                description: "Contexte partagé de test".into()
            }
        )
        .await
        .is_err()
    );
    for _ in 1..10 {
        create(&actor, Uuid::new_v4()).await?;
    }
    assert!(matches!(
        company::create(
            &actor,
            CreateCompany {
                public_id: Uuid::new_v4(),
                name: "[FICTIF] Exceeds limit".into(),
                description: String::new()
            }
        )
        .await,
        Err(AppError::Invalid(_))
    ));
    let workspaces = service::list_workspaces(&actor).await?;
    assert_eq!(workspaces.len(), 10);
    actor.pool.close().await;
    Ok(())
}

// One scenario retains both tenants and the exact graph identities throughout.
#[allow(clippy::too_many_lines)]
#[tokio::test]
async fn scoped_graph_keeps_real_relations_and_rejects_foreign_objects() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let other = create(&actor, Uuid::new_v4()).await?;
    let a = service::create_project(
        &owner,
        CreateProject {
            name: "[FICTIF] Projet A".into(),
            objective: "Contexte A".into(),
        },
    )
    .await?;
    let b = service::create_project(
        &owner,
        CreateProject {
            name: "[FICTIF] Projet B".into(),
            objective: "Contexte B".into(),
        },
    )
    .await?;
    let foreign = service::create_project(
        &other,
        CreateProject {
            name: "[FICTIF] Projet secret".into(),
            objective: "Autre société".into(),
        },
    )
    .await?;
    assert_eq!(service::list_projects(&owner).await?.len(), 2);
    let snapshot = service::snapshot(&owner, a.public_id).await?;
    assert_eq!(snapshot.nodes.len(), 5);
    for node_key in ["tech", "dev"] {
        assert!(
            matches!(
                service::create_session(
                    &owner,
                    a.public_id,
                    CreateSession {
                        node_key: node_key.into(),
                        title: None
                    }
                )
                .await,
                Err(AppError::Invalid(_))
            ),
            "technical agents require a handoff pack"
        );
    }
    for node_key in ["general", "sales"] {
        service::create_session(
            &owner,
            a.public_id,
            CreateSession {
                node_key: node_key.into(),
                title: None,
            },
        )
        .await?;
    }
    let edge_id = Uuid::new_v4();
    let edge = company::create_edge(
        &owner,
        CreateGraphEdge {
            public_id: edge_id,
            source: GraphEndpoint {
                kind: "scope".into(),
                public_id: a.public_id,
                project_public_id: a.public_id,
            },
            target: GraphEndpoint {
                kind: "scope".into(),
                public_id: b.public_id,
                project_public_id: b.public_id,
            },
            edge_type: "depends_on".into(),
        },
        None,
    )
    .await?;
    assert_eq!(edge.status, "confirmed");
    let graph = company::graph(
        &owner,
        GraphQuery {
            project_id: Some(a.public_id),
            limit: None,
        },
    )
    .await?;
    assert!(
        graph.nodes.iter().any(|node| node.id == b.public_id),
        "one-hop linked scope must be projected"
    );
    assert!(graph.edges.iter().any(|candidate| candidate.id == edge_id));
    assert!(!graph.nodes.iter().any(|node| node.id == foreign.public_id));
    for edge in &graph.edges {
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.id == edge.source_public_id && node.kind == edge.source_kind)
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.id == edge.target_public_id && node.kind == edge.target_kind)
        );
    }
    assert!(matches!(
        company::graph(
            &owner,
            GraphQuery {
                project_id: Some(foreign.public_id),
                limit: None
            }
        )
        .await,
        Err(AppError::NotFound)
    ));
    assert!(matches!(
        company::create_edge(
            &owner,
            CreateGraphEdge {
                public_id: Uuid::new_v4(),
                source: GraphEndpoint {
                    kind: "scope".into(),
                    public_id: a.public_id,
                    project_public_id: a.public_id
                },
                target: GraphEndpoint {
                    kind: "scope".into(),
                    public_id: foreign.public_id,
                    project_public_id: foreign.public_id
                },
                edge_type: "references".into()
            },
            None
        )
        .await,
        Err(AppError::NotFound)
    ));
    let bounded = company::graph(
        &owner,
        GraphQuery {
            project_id: None,
            limit: Some(1),
        },
    )
    .await?;
    assert!(bounded.truncated);
    assert_eq!(bounded.nodes.len(), 1);
    assert!(bounded.edges.is_empty());
    owner.pool.close().await;
    Ok(())
}

#[tokio::test]
async fn company_roles_and_last_owner_are_enforced_and_revocation_is_effective() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let overview = company::overview(&owner).await?;
    let owner_member = overview
        .members
        .iter()
        .find(|member| member.actor_id == owner.actor_id)
        .context("owner")?;
    assert!(matches!(
        company::update_member(
            &owner,
            owner_member.public_id,
            UpdateMember {
                role: Some("viewer".into()),
                invitation_status: None
            },
            None
        )
        .await,
        Err(AppError::Conflict(_))
    ));
    let viewer_actor = Uuid::new_v4();
    let viewer_member = add_member(&owner, viewer_actor, "viewer").await?;
    let viewer = select(&owner, owner.workspace_id, viewer_actor).await?;
    assert!(company::overview(&viewer).await.is_ok());
    assert!(matches!(
        company::update(
            &viewer,
            CompanyInput {
                name: "Forbidden".into(),
                description: String::new()
            },
            false,
            None
        )
        .await,
        Err(AppError::Forbidden)
    ));
    assert!(matches!(
        company::update_member(
            &viewer,
            viewer_member,
            UpdateMember {
                role: Some("owner".into()),
                invitation_status: None
            },
            None
        )
        .await,
        Err(AppError::Forbidden)
    ));
    let updated = company::update_member(
        &owner,
        viewer_member,
        UpdateMember {
            role: None,
            invitation_status: Some("revoked".into()),
        },
        None,
    )
    .await?;
    assert_eq!(updated.invitation_status, "revoked");
    let auth = AuthRuntime::new(AuthMode::Local, owner.workspace_id, viewer_actor, None)?;
    assert!(matches!(
        auth.authenticate(&axum::http::HeaderMap::new(), &owner.pool)
            .await,
        Err(AppError::Forbidden)
    ));
    assert!(
        company::overview(&viewer).await.is_err(),
        "cached request context cannot bypass membership RLS"
    );
    let updated = company::update(
        &owner,
        CompanyInput {
            name: "[FICTIF] Société renommée".into(),
            description: "Description mise à jour".into(),
        },
        true,
        None,
    )
    .await?;
    assert_eq!(updated.agents.len(), 5);
    assert_eq!(updated.workspace.name, "[FICTIF] Société renommée");
    owner.pool.close().await;
    Ok(())
}

async fn bootstrap_server(state: &AppState) -> Result<(String, tokio::task::JoinHandle<()>)> {
    let auth = Arc::new(AuthRuntime::new(
        AuthMode::Local,
        Uuid::nil(),
        state.actor_id,
        None,
    )?);
    let app = routes::router(
        Arc::new(state.clone()),
        auth,
        None,
        vec!["http://127.0.0.1".parse()?],
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Ok((format!("http://{address}"), task))
}

#[tokio::test]
async fn authenticated_actor_can_bootstrap_through_http_without_a_workspace() -> Result<()> {
    let state = isolated_actor().await?;
    let (base, task) = bootstrap_server(&state).await?;
    let client = reqwest::Client::new();
    let id = Uuid::new_v4();
    let body = json!({"public_id":id,"name":"[FICTIF] HTTP bootstrap","description":""});
    let rejected = client
        .post(format!("{base}/api/workspaces"))
        .json(&body)
        .send()
        .await?;
    assert_eq!(rejected.status(), 422);
    let created = client
        .post(format!("{base}/api/workspaces"))
        .header("Idempotency-Key", Uuid::new_v4().to_string())
        .json(&body)
        .send()
        .await?;
    assert_eq!(created.status(), 200);
    let value: serde_json::Value = created.json().await?;
    assert_eq!(value["public_id"], id.to_string());
    assert_eq!(value["role"], "owner");
    task.abort();
    state.pool.close().await;
    Ok(())
}

async fn fixture_knowledge(
    owner: &AppState,
    project: Uuid,
    entry_type: &str,
    statement: &str,
) -> Result<(Uuid, Uuid)> {
    let mut tx = owner.pool.begin().await?;
    sqlx::query("select set_config('app.current_actor_id',$1,true),set_config('app.current_workspace_id',$2,true),set_config('app.current_workspace_role','owner',true)")
        .bind(owner.actor_id.to_string()).bind(owner.workspace_internal_id.context("scope")?.to_string()).execute(&mut *tx).await?;
    let (project_id, node_id): (i64, i64) = sqlx::query_as(
        "select p.id,n.id from app.projects p join app.context_nodes n on n.project_id=p.id
        where p.public_id=$1 and n.node_key='product'",
    )
    .bind(project)
    .fetch_one(&mut *tx)
    .await?;
    let (entry, entry_public): (i64, Uuid) = sqlx::query_as(
        "insert into app.knowledge_entries(workspace_id,project_id,context_node_id,entry_type)
        values(app.current_workspace_id(),$1,$2,$3) returning id,public_id",
    )
    .bind(project_id)
    .bind(node_id)
    .bind(entry_type)
    .fetch_one(&mut *tx)
    .await?;
    let version=sqlx::query_scalar("insert into app.knowledge_entry_versions(knowledge_entry_id,workspace_id,project_id,context_node_id,version_number,
        entry_type,title,statement,rationale,author_actor_id,origin_type)
        values($1,app.current_workspace_id(),$2,$3,1,$4,'[FICTIF] Source contrôlée',$5,'Fixture synthétique',app.current_actor_id(),'manual') returning public_id")
        .bind(entry).bind(project_id).bind(node_id).bind(entry_type).bind(statement).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok((entry_public, version))
}

async fn ready_project(owner: &AppState, name: &str) -> Result<(Uuid, Uuid)> {
    let project = service::create_project(
        owner,
        CreateProject {
            name: name.into(),
            objective: "[FICTIF] Valider et tracer les décisions".into(),
        },
    )
    .await?;
    for (kind, statement) in [
        (
            "business_rule",
            "Une décision reste traçable après validation humaine.",
        ),
        ("requirement", "Afficher les décisions confirmées."),
        (
            "acceptance_criterion",
            "Une décision confirmée apparaît dans la liste avec sa preuve.",
        ),
    ] {
        fixture_knowledge(owner, project.public_id, kind, statement).await?;
    }
    let session = service::create_session(
        owner,
        project.public_id,
        CreateSession {
            node_key: "product".into(),
            title: None,
        },
    )
    .await?;
    Ok((project.public_id, session.session.public_id))
}

async fn compile_for(
    owner: &AppState,
    project: Uuid,
    session: Uuid,
    target: &str,
) -> Result<ai_center_server::models::ContextPackSummary> {
    Ok(service::compile_context_pack(
        owner,
        project,
        ai_center_server::models::CompileContextPack {
            source_session_id: session,
            target_node_key: Some(target.into()),
            task_kind: "technical-delivery-plan".into(),
            token_budget: None,
        },
    )
    .await?)
}

async fn link_projects(owner: &AppState, source: Uuid, target: Uuid) -> Result<()> {
    company::create_edge(
        owner,
        CreateGraphEdge {
            public_id: Uuid::new_v4(),
            source: GraphEndpoint {
                kind: "scope".into(),
                public_id: source,
                project_public_id: source,
            },
            target: GraphEndpoint {
                kind: "scope".into(),
                public_id: target,
                project_public_id: target,
            },
            edge_type: "depends_on".into(),
        },
        None,
    )
    .await?;
    Ok(())
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // One scenario follows exact provenance across compile, relay, export and revision.
async fn scoped_sources_are_versioned_deduplicated_and_invalidate_only_dependents() -> Result<()> {
    use ai_center_server::models::{CreateHandoff, ReviseKnowledge, SendMessage};
    use sha2::{Digest, Sha256};
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let company_scope = company::overview(&owner)
        .await?
        .company_scope
        .context("company scope")?
        .project_public_id;
    let (_, company_rule) = fixture_knowledge(
        &owner,
        company_scope,
        "business_rule",
        "[FICTIF] Toutes les décisions demandent un accord humain.",
    )
    .await?;
    let (a, product) = ready_project(&owner, "[FICTIF] Projet dépendant").await?;
    let (b, _) = ready_project(&owner, "[FICTIF] Projet source").await?;
    let (c, _) = ready_project(&owner, "[FICTIF] Source à distance deux").await?;
    let (_, depth_two) = fixture_knowledge(
        &owner,
        c,
        "constraint",
        "[FICTIF] Limiter les exports aux membres de la société.",
    )
    .await?;
    let (related, related_version) = fixture_knowledge(
        &owner,
        b,
        "constraint",
        "[FICTIF] Conserver les décisions sans expiration.",
    )
    .await?;
    let (unrelated, _) = fixture_knowledge(
        &owner,
        b,
        "technical_rule",
        "[FICTIF] Ancienne couleur du panneau.",
    )
    .await?;
    link_projects(&owner, a, b).await?;
    link_projects(&owner, b, c).await?;
    let pack = compile_for(&owner, a, product, "tech").await?;
    assert_eq!(pack.compiler_version, "company-scoped-context-v2");
    let included: Vec<Uuid> = pack.content["knowledge"]
        .as_array()
        .context("knowledge")?
        .iter()
        .filter_map(|source| source["version_public_id"].as_str()?.parse().ok())
        .collect();
    for version in [company_rule, related_version, depth_two] {
        assert!(included.contains(&version));
    }
    let unique: std::collections::HashSet<_> = included.iter().collect();
    assert_eq!(included.len(), unique.len());
    assert_eq!(
        pack.content_hash,
        format!("{:x}", Sha256::digest(serde_json::to_vec(&pack.content)?))
    );
    let exported = service::export_context_pack(&owner, pack.public_id, "json").await?;
    assert!(exported.1.contains(&related_version.to_string()));
    assert!(exported.1.contains("source_project_public_id"));
    assert!(
        pack.selection_items
            .iter()
            .any(|item| item.candidate_public_id == related_version && item.decision == "included")
    );
    let handoff = service::create_handoff(
        &owner,
        a,
        CreateHandoff {
            source_session_id: product,
            context_pack_id: pack.public_id,
        },
    )
    .await?;
    let dev = compile_for(&owner, a, product, "dev").await?;
    let dev_handoff = service::create_handoff(
        &owner,
        a,
        CreateHandoff {
            source_session_id: handoff.target_session_public_id,
            context_pack_id: dev.public_id,
        },
    )
    .await?;
    let dev_session = service::session_view(&owner, dev_handoff.target_session_public_id).await?;
    assert_eq!(dev_session.session.node_key, "dev");
    service::revise_knowledge_for_project(
        &owner,
        b,
        unrelated,
        ReviseKnowledge {
            title: None,
            statement: "[FICTIF] Nouvelle couleur du panneau.".into(),
            rationale: None,
        },
    )
    .await?;
    let unchanged = service::context_pack(&owner, pack.public_id).await?;
    assert_eq!(
        unchanged.status, "current",
        "excluded source revision must leave dependent-free pack usable"
    );
    service::send_message(
        &owner,
        handoff.target_session_public_id,
        SendMessage {
            client_message_id: Uuid::new_v4(),
            content: "[FICTIF] Comprendre les contraintes.".into(),
        },
    )
    .await?;
    let graph = company::graph(
        &owner,
        GraphQuery {
            project_id: Some(a),
            limit: None,
        },
    )
    .await?;
    assert!(
        graph
            .edges
            .iter()
            .any(|edge| edge.source_public_id == pack.public_id
                && edge.target_public_id == related_version)
    );
    service::revise_knowledge_for_project(
        &owner,
        b,
        related,
        ReviseKnowledge {
            title: None,
            statement: "[FICTIF] Conserver les décisions cent ans après accord humain.".into(),
            rationale: None,
        },
    )
    .await?;
    let stale = service::context_pack(&owner, pack.public_id).await?;
    assert_eq!(stale.status, "stale");
    assert_eq!(stale.content, pack.content);
    assert_eq!(stale.content_hash, pack.content_hash);
    assert!(matches!(
        service::send_message(
            &owner,
            dev_handoff.target_session_public_id,
            SendMessage {
                client_message_id: Uuid::new_v4(),
                content: "[FICTIF] Continuer".into()
            }
        )
        .await,
        Err(AppError::Conflict(_))
    ));
    Ok(())
}

#[tokio::test]
async fn validated_artifact_sources_survive_drafts_then_expire_on_new_validation() -> Result<()> {
    use ai_center_server::artifacts::{
        self,
        models::{CreateArtifact, SaveDraft, ValidateArtifact},
    };
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let (a, product) = ready_project(&owner, "[FICTIF] Consommateur de spécification").await?;
    let (b, _) = ready_project(&owner, "[FICTIF] Source de spécification").await?;
    link_projects(&owner, a, b).await?;
    let draft = artifacts::create(
        &owner,
        b,
        CreateArtifact {
            artifact_type: "specification".into(),
            title: "[FICTIF] Spécification".into(),
            body_markdown: "Conserver chaque décision confirmée et son origine.".into(),
            structured_content: json!({}),
            sources: vec![],
        },
        None,
    )
    .await?;
    let validated = artifacts::validate(
        &owner,
        draft.artifact.public_id,
        ValidateArtifact {
            expected_version_id: draft.current_version.public_id,
        },
        None,
    )
    .await?;
    let pack = compile_for(&owner, a, product, "tech").await?;
    assert!(
        pack.selection_items
            .iter()
            .any(
                |item| item.candidate_public_id == validated.current_version.public_id
                    && item.decision == "included"
            )
    );
    let next = artifacts::save_draft(
        &owner,
        draft.artifact.public_id,
        SaveDraft {
            expected_version_id: validated.current_version.public_id,
            title: "[FICTIF] Nouvelle spécification".into(),
            body_markdown: "Conserver chaque décision et demander un accord explicite.".into(),
            structured_content: json!({}),
            sources: vec![],
        },
        None,
    )
    .await?;
    assert_eq!(
        service::context_pack(&owner, pack.public_id).await?.status,
        "current"
    );
    let updated = artifacts::validate(
        &owner,
        draft.artifact.public_id,
        ValidateArtifact {
            expected_version_id: next.current_version.public_id,
        },
        None,
    )
    .await?;
    assert_eq!(
        service::context_pack(&owner, pack.public_id).await?.status,
        "stale"
    );
    let fresh = compile_for(&owner, a, product, "dev").await?;
    assert!(
        fresh
            .selection_items
            .iter()
            .any(
                |item| item.candidate_public_id == updated.current_version.public_id
                    && item.decision == "included"
            )
    );
    assert!(
        !fresh
            .selection_items
            .iter()
            .any(|item| item.candidate_public_id == validated.current_version.public_id)
    );
    assert_eq!(
        pack.content,
        service::context_pack(&owner, pack.public_id).await?.content
    );
    Ok(())
}

#[tokio::test]
async fn company_steward_cites_cross_project_versions_and_keeps_human_status_separate() -> Result<()>
{
    use ai_center_server::{
        models::ReviseKnowledge,
        steward::{self, StewardConfig},
    };
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let a = service::create_project(
        &owner,
        CreateProject {
            name: "[FICTIF] Politique entreprise".into(),
            objective: "Conservation des décisions".into(),
        },
    )
    .await?;
    let b = service::create_project(
        &owner,
        CreateProject {
            name: "[FICTIF] Décision technique".into(),
            objective: "Conservation des décisions".into(),
        },
    )
    .await?;
    let (rule, rule_version) = fixture_knowledge(
        &owner,
        a.public_id,
        "business_rule",
        "[FICTIF] Conserver les décisions sans expiration.",
    )
    .await?;
    let (_, tech_version) = fixture_knowledge(
        &owner,
        b.public_id,
        "technical_rule",
        "[FICTIF] Purger les décisions après 90 jours.",
    )
    .await?;
    let first = steward::analyze_project(&owner, b.public_id, StewardConfig::default()).await?;
    assert_eq!(first.contradiction_count, 1);
    let insight_id = *first
        .insight_public_ids
        .first()
        .context("cross-project contradiction")?;
    let insight = service::insight_detail(&owner, insight_id).await?;
    assert_eq!(insight.insight.source_status.as_deref(), Some("current"));
    assert_eq!(insight.sources.len(), 2);
    for (version, project) in [(rule_version, a.public_id), (tech_version, b.public_id)] {
        assert!(
            insight
                .sources
                .iter()
                .any(|source| source.version_public_id == Some(version)
                    && source.source_project_public_id == Some(project))
        );
    }
    // Replayed exact versions must not create another notification.
    let replay = steward::analyze_project(&owner, a.public_id, StewardConfig::default()).await?;
    assert_eq!(replay.insight_public_ids, vec![insight_id]);
    let graph = company::graph(
        &owner,
        GraphQuery {
            project_id: None,
            limit: None,
        },
    )
    .await?;
    assert!(
        graph.edges.iter().any(
            |edge| edge.source_public_id == insight_id && edge.target_public_id == rule_version
        )
    );
    service::revise_knowledge_for_project(
        &owner,
        a.public_id,
        rule,
        ReviseKnowledge {
            title: None,
            statement: "[FICTIF] Purger les décisions après 90 jours, après accord humain.".into(),
            rationale: None,
        },
    )
    .await?;
    let historical = service::insight_detail(&owner, insight_id).await?;
    assert_eq!(historical.insight.source_status.as_deref(), Some("stale"));
    assert_eq!(
        historical.insight.status, "open",
        "freshness never rewrites a human disposition"
    );
    assert!(
        historical
            .sources
            .iter()
            .any(|source| source.version_public_id == Some(rule_version))
    );
    let current = steward::analyze_project(&owner, b.public_id, StewardConfig::default()).await?;
    assert_eq!(current.contradiction_count, 0);
    let other = create(&actor, Uuid::new_v4()).await?;
    assert!(matches!(
        service::insight_detail(&other, insight_id).await,
        Err(AppError::NotFound)
    ));
    Ok(())
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Source insufficiency and archived history form one lifecycle.
async fn github_metadata_creates_an_unknown_context_gap_and_not_code_conformity() -> Result<()> {
    use ai_center_server::steward::{self, StewardConfig};
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let project = service::create_project(
        &owner,
        CreateProject {
            name: "[FICTIF] Métadonnées GitHub".into(),
            objective: "Conservation des décisions".into(),
        },
    )
    .await?;
    fixture_knowledge(
        &owner,
        project.public_id,
        "business_rule",
        "[FICTIF] Conserver les décisions sans expiration.",
    )
    .await?;
    let mut tx = owner.pool.begin().await?;
    sqlx::query("select set_config('app.current_actor_id',$1,true),set_config('app.current_workspace_id',$2,true),set_config('app.current_workspace_role','owner',true)")
        .bind(owner.actor_id.to_string()).bind(owner.workspace_internal_id.context("scope")?.to_string()).execute(&mut *tx).await?;
    let internal: i64 = sqlx::query_scalar("select id from app.projects where public_id=$1")
        .bind(project.public_id)
        .fetch_one(&mut *tx)
        .await?;
    // Connector provisioning is intentionally unavailable to the runtime role.
    // Only this fixture row uses the guarded disposable database administrator.
    let admin_raw = std::env::var("AI_CENTER_ADMIN_DATABASE_URL")?;
    let admin_url =
        url::Url::parse(&admin_raw).map_err(|_| anyhow::anyhow!("invalid isolated admin URL"))?;
    ensure!(
        admin_url.scheme() == "postgresql"
            && admin_url.host_str() == Some("127.0.0.1")
            && admin_url.port() == Some(55322)
            && admin_url.username() == "postgres"
            && admin_url.path() == "/postgres"
            && admin_url.query().is_none()
            && admin_url.fragment().is_none(),
        "isolated admin database required"
    );
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin_raw)
        .await?;
    let connection:i64=sqlx::query_scalar("insert into app.tool_connections(workspace_id,provider,auth_mode,external_account_id,display_name,secret_reference,created_by_actor_id)
        values($1,'github','github_app','fictif','[FICTIF] Aucun provider','synthetic-placeholder',$2) returning id")
        .bind(owner.workspace_internal_id).bind(owner.actor_id).fetch_one(&admin).await?;
    let reference:i64=sqlx::query_scalar("insert into app.external_references(workspace_id,project_id,tool_connection_id,provider,object_kind,external_id,canonical_url,repository_full_name,display_title,created_by_actor_id)
        values(app.current_workspace_id(),$1,$2,'github','pull_request','fictif-pr-1','https://github.com/fictif/test/pull/1','fictif/test','[FICTIF] Purger les décisions après 90 jours',app.current_actor_id()) returning id")
        .bind(internal).bind(connection).fetch_one(&mut *tx).await?;
    let observation:Uuid=sqlx::query_scalar("insert into app.external_reference_observations(workspace_id,project_id,external_reference_id,observation_status,content_hash,observed_state)
        values(app.current_workspace_id(),$1,$2,'current',$3,$4) returning public_id")
        .bind(internal).bind(reference).bind("0".repeat(64)).bind(json!({"title":"[FICTIF] Purger après 90 jours","checks":"success","code_read":false}))
        .fetch_one(&mut *tx).await?;
    tx.commit().await?;
    let result =
        steward::analyze_project(&owner, project.public_id, StewardConfig::default()).await?;
    assert_eq!(
        result.contradiction_count, 0,
        "metadata cannot substantiate a model contradiction"
    );
    let insight_id = *result
        .insight_public_ids
        .first()
        .context("inspection gap")?;
    let detail = service::insight_detail(&owner, insight_id).await?;
    assert_eq!(detail.insight.insight_type, "context_gap");
    assert_eq!(detail.insight.source_status.as_deref(), Some("unknown"));
    assert!(detail.insight.confidence.abs() < f64::EPSILON);
    assert!(
        detail
            .insight
            .explanation
            .contains("explicitement signalés comme lus")
    );
    assert!(
        detail
            .sources
            .iter()
            .any(|source| source.object_public_id == observation
                && source.object_kind == "external_reference_observation")
    );
    let graph = company::graph(
        &owner,
        GraphQuery {
            project_id: Some(project.public_id),
            limit: None,
        },
    )
    .await?;
    assert!(
        graph
            .nodes
            .iter()
            .any(|node| node.id == observation && node.kind == "external_reference")
    );
    assert!(graph.edges.iter().any(|edge|edge.source_public_id==insight_id && edge.target_public_id==observation));
    company::data::archive(
        &owner,
        project.public_id,
        company::data::ArchiveProject { archived: true },
        None,
    )
    .await?;
    assert!(matches!(
        service::act_on_insight(
            &owner,
            insight_id,
            ai_center_server::models::InsightAction {
                action: ai_center_server::models::InsightDecision::Dismiss,
                justification: "[FICTIF] Projet archivé".into(),
            }
        )
        .await,
        Err(AppError::Conflict(_))
    ));
    assert!(service::insight_detail(&owner, insight_id).await.is_ok());
    Ok(())
}

#[tokio::test]
async fn owner_export_preserves_history_and_archive_is_reversible_but_never_revalidates_packs()
-> Result<()> {
    use ai_center_server::company::data::{self, ArchiveProject};
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let (project, session) = ready_project(&owner, "[FICTIF] Données portables").await?;
    let pack = compile_for(&owner, project, session, "tech").await?;
    let archived = data::archive(&owner, project, ArchiveProject { archived: true }, None).await?;
    assert_eq!(archived.status, "archived");
    assert_eq!(data::archived_projects(&owner).await?[0].public_id, project);
    assert!(
        !service::list_projects(&owner)
            .await?
            .iter()
            .any(|item| item.public_id == project)
    );
    assert_eq!(
        service::context_pack(&owner, pack.public_id).await?.status,
        "stale"
    );
    assert!(matches!(
        service::create_session(
            &owner,
            project,
            CreateSession {
                node_key: "general".into(),
                title: None
            }
        )
        .await,
        Err(AppError::Conflict(_))
    ));
    assert!(
        service::session_view(&owner, session).await.is_ok(),
        "archival preserves readable history"
    );
    let replay = data::archive(&owner, project, ArchiveProject { archived: true }, None).await?;
    assert_eq!(replay.graph_version, archived.graph_version);
    let exported = data::export(&owner).await?;
    assert!(exported.complete);
    assert_eq!(exported.workspace_public_id, owner.workspace_id);
    assert_eq!(exported.data["knowledge_entry_versions"].len(), 3);
    assert_eq!(
        exported.data["context_packs"][0]["content_hash"],
        pack.content_hash
    );
    assert!(
        exported.data["projects"]
            .iter()
            .any(|row| row["public_id"] == project.to_string() && row["status"] == "archived")
    );
    assert!(!exported.data.contains_key("provider_connections"));
    assert!(!exported.data.contains_key("work_tool_connections"));
    assert!(!exported.data.contains_key("workspace_invitations"));
    let viewer = Uuid::new_v4();
    add_member(&owner, viewer, "viewer").await?;
    let view_state = select(&owner, owner.workspace_id, viewer).await?;
    assert!(matches!(
        data::archived_projects(&view_state).await,
        Err(AppError::Forbidden)
    ));
    assert!(matches!(
        data::export(&view_state).await,
        Err(AppError::Forbidden)
    ));
    assert!(matches!(
        data::archive(
            &view_state,
            project,
            ArchiveProject { archived: false },
            None
        )
        .await,
        Err(AppError::Forbidden)
    ));
    let restored = data::archive(&owner, project, ArchiveProject { archived: false }, None).await?;
    assert_eq!(restored.status, "active");
    assert!(data::archived_projects(&owner).await?.is_empty());
    assert_eq!(
        service::context_pack(&owner, pack.public_id).await?.status,
        "stale"
    );
    let foreign = create(&actor, Uuid::new_v4()).await?;
    assert!(matches!(
        data::archive(&foreign, project, ArchiveProject { archived: true }, None).await,
        Err(AppError::NotFound)
    ));
    assert!(data::export(&foreign).await?.data["knowledge_entry_versions"].is_empty());
    Ok(())
}

#[tokio::test]
async fn concurrent_graph_relationships_lock_scopes_in_one_order() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let a = service::create_project(
        &owner,
        CreateProject {
            name: "[FICTIF] Lock A".into(),
            objective: "Graph concurrency".into(),
        },
    )
    .await?;
    let b = service::create_project(
        &owner,
        CreateProject {
            name: "[FICTIF] Lock B".into(),
            objective: "Graph concurrency".into(),
        },
    )
    .await?;
    let mut tasks = tokio::task::JoinSet::new();
    for (i, relation) in [
        "references",
        "depends_on",
        "informs",
        "supersedes",
        "contradicts",
        "evidenced_by",
    ]
    .into_iter()
    .enumerate()
    {
        let owner = owner.clone();
        let (source, target) = if i % 2 == 0 {
            (a.public_id, b.public_id)
        } else {
            (b.public_id, a.public_id)
        };
        tasks.spawn(async move {
            company::create_edge(
                &owner,
                CreateGraphEdge {
                    public_id: Uuid::new_v4(),
                    source: GraphEndpoint {
                        kind: "scope".into(),
                        public_id: source,
                        project_public_id: source,
                    },
                    target: GraphEndpoint {
                        kind: "scope".into(),
                        public_id: target,
                        project_public_id: target,
                    },
                    edge_type: relation.into(),
                },
                None,
            )
            .await
        });
    }
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(result) = tasks.join_next().await {
            result??;
        }
        anyhow::Ok(())
    })
    .await??;
    Ok(())
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Exact source lifecycle across two immutable observations.
async fn steward_code_sources_keep_exact_commit_lines_and_mark_partial_evidence_unknown()
-> Result<()> {
    use ai_center_server::steward::{self, StewardConfig};
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let project = service::create_project(
        &owner,
        CreateProject {
            name: "[FICTIF] Code observé".into(),
            objective: "Conservation des décisions".into(),
        },
    )
    .await?;
    fixture_knowledge(
        &owner,
        project.public_id,
        "business_rule",
        "[FICTIF] Conserver les décisions sans expiration.",
    )
    .await?;
    let mut tx = owner.pool.begin().await?;
    sqlx::query("select set_config('app.current_actor_id',$1,true),set_config('app.current_workspace_id',$2,true),set_config('app.current_workspace_role','owner',true)")
        .bind(owner.actor_id.to_string()).bind(owner.workspace_internal_id.context("scope")?.to_string()).execute(&mut *tx).await?;
    let internal: i64 = sqlx::query_scalar("select id from app.projects where public_id=$1")
        .bind(project.public_id)
        .fetch_one(&mut *tx)
        .await?;
    let connection:i64=sqlx::query_scalar("insert into app.work_tool_connections(public_id,workspace_id,provider,name,encrypted_credential,credential_actor_id) values($1,app.current_workspace_id(),'github','[FICTIF] Non utilisable',$2,app.current_actor_id()) returning id")
        .bind(Uuid::new_v4()).bind(vec![0_u8;32]).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    let mut previous_insight = None;
    for partial in [false, true] {
        let commit = if partial {
            "b".repeat(40)
        } else {
            "a".repeat(40)
        };
        let content = if partial {
            format!(
                "{:<5999}\nsecond line",
                "// [FICTIF] Purger les décisions après 90 jours."
            )
        } else {
            "// [FICTIF] Purger les décisions après 90 jours.\nconst RETENTION_DAYS = 90;\n".into()
        };
        let mut tx = owner.pool.begin().await?;
        sqlx::query("select set_config('app.current_actor_id',$1,true),set_config('app.current_workspace_id',$2,true),set_config('app.current_workspace_role','owner',true)")
            .bind(owner.actor_id.to_string()).bind(owner.workspace_internal_id.context("scope")?.to_string()).execute(&mut *tx).await?;
        let corpus:i64=sqlx::query_scalar("insert into app.github_code_corpora(workspace_id,project_id,connection_id,repository,commit_sha,commit_verified,requested_paths,requested_by_actor_id) values(app.current_workspace_id(),$1,$2,'fictif/retention',$3,true,'[\"retention.ts\"]',app.current_actor_id()) returning id")
            .bind(internal).bind(connection).bind(&commit).fetch_one(&mut *tx).await?;
        let observation:Uuid=sqlx::query_scalar("insert into app.github_code_file_observations(workspace_id,project_id,corpus_id,path,status,blob_sha,content_hash,content_text,line_count) values(app.current_workspace_id(),$1,$2,'retention.ts','code_read',$3,$4,$5,2) returning public_id")
            .bind(internal).bind(corpus).bind("c".repeat(40)).bind("d".repeat(64)).bind(content).fetch_one(&mut *tx).await?;
        tx.commit().await?;
        if let Some(old) = previous_insight {
            assert_eq!(
                service::insight_detail(&owner, old)
                    .await?
                    .insight
                    .source_status
                    .as_deref(),
                Some("stale")
            );
        }
        let result =
            steward::analyze_project(&owner, project.public_id, StewardConfig::default()).await?;
        let id = *result
            .insight_public_ids
            .first()
            .context("code evidence insight")?;
        let detail = service::insight_detail(&owner, id).await?;
        assert_eq!(
            detail.insight.source_status.as_deref(),
            Some(if partial { "unknown" } else { "current" })
        );
        let source = detail
            .sources
            .iter()
            .find(|s| s.object_public_id == observation)
            .context("typed file source")?;
        assert_eq!(source.object_kind, "github_code_file_observation");
        let provenance = source
            .provenance
            .as_ref()
            .context("exact code provenance")?;
        assert_eq!(provenance["commit_sha"], commit);
        assert_eq!(provenance["line_start"], 1);
        assert_eq!(provenance["line_end"], if partial { 1 } else { 2 });
        assert_eq!(provenance["excerpt_partial"], partial);
        assert_eq!(provenance["repository_coverage"], "selected_file_only");
        if partial {
            assert_eq!(detail.insight.insight_type, "context_gap");
        }
        let graph = company::graph(
            &owner,
            GraphQuery {
                project_id: Some(project.public_id),
                limit: None,
            },
        )
        .await?;
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.id == observation && node.kind == "external_reference")
        );
        assert!(
            graph
                .edges
                .iter()
                .any(|edge| edge.source_public_id == id && edge.target_public_id == observation)
        );
        previous_insight = Some(id);
    }
    Ok(())
}

#[path = "company_context/abandoned.rs"]
mod abandoned;
#[path = "company_context/erasure.rs"]
mod erasure;
#[path = "company_context/project_data.rs"]
mod project_data;
#[path = "company_context/project_lifecycle.rs"]
mod project_lifecycle;
#[path = "company_context/retrieval.rs"]
mod retrieval;

#[path = "company_context/graph_navigation.rs"]
mod graph_navigation;

#[path = "company_context/knowledge_library.rs"]
mod knowledge_library;
