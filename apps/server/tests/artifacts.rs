//! `PostgreSQL` and HTTP acceptance for manual artifacts; all data is fictitious.
//! Run only through the checkout-isolated integration stack, never a user DB.
use std::sync::Arc;

use ai_center_server::{
    agent::DeterministicEngine,
    artifacts::{
        self, ArtifactDetail, CreateArtifact, ListArtifacts, ResetDestination, SaveDraft,
        SetDestination, SourceInput, ValidateArtifact,
    },
    auth::AuthRuntime,
    config::AuthMode,
    error::AppError,
    models::{CreateProject, CreateSession},
    routes,
    service::{self, AppState},
};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Transaction, postgres::PgPoolOptions};
use uuid::Uuid;

struct Fixture {
    pool: PgPool,
    admin: PgPool,
    owner: AppState,
    viewer: AppState,
    foreign: AppState,
}

fn isolated_urls() -> Result<(String, String)> {
    let runtime = std::env::var("DATABASE_URL")?;
    let admin = std::env::var("AI_CENTER_ADMIN_DATABASE_URL")?;
    for (raw, role) in [(&runtime, "ai_center_runtime"), (&admin, "postgres")] {
        let url = url::Url::parse(raw).map_err(|_| anyhow::anyhow!("invalid integration URL"))?;
        ensure!(
            url.scheme() == "postgresql"
                && url.host_str() == Some("127.0.0.1")
                && url.port() == Some(55322)
                && url.username() == role
                && url.path() == "/postgres"
                && url.query().is_none()
                && url.fragment().is_none(),
            "Only the isolated integration database on 55322 is permitted"
        );
    }
    ensure!(
        std::env::var("AI_CENTER_EXPECT_DATABASE_ROLE").as_deref() == Ok("ai_center_runtime")
            && std::env::var("AI_CENTER_AGENT_MODE").as_deref() == Ok("deterministic"),
        "Explicit isolated runtime and deterministic mode are required"
    );
    Ok((runtime, admin))
}
async fn fixture() -> Result<Fixture> {
    let (runtime_url, admin_url) = isolated_urls()?;
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&runtime_url)
        .await?;
    let admin = PgPoolOptions::new()
        .max_connections(2)
        .connect(&admin_url)
        .await?;
    let bypass: bool = sqlx::query_scalar(
        "select rolsuper or rolbypassrls from pg_roles where rolname=current_user",
    )
    .fetch_one(&pool)
    .await?;
    ensure!(
        !bypass,
        "Artifact acceptance requires an unprivileged RLS runtime"
    );
    let owner = Uuid::new_v4();
    let viewer = Uuid::new_v4();
    let mut workspaces = Vec::new();
    for _ in 0..2 {
        let (id,public_id):(i64,Uuid)=sqlx::query_as("insert into app.workspaces(owner_actor_id,name) values($1,'[FICTIF] Artefacts') returning id,public_id")
            .bind(owner).fetch_one(&admin).await?;
        for (actor, role) in [(owner, "owner"), (viewer, "viewer")] {
            sqlx::query("insert into app.workspace_members(workspace_id,actor_id,role,invitation_status,accepted_at) values($1,$2,$3,'accepted',now())")
                .bind(id).bind(actor).bind(role).execute(&admin).await?;
        }
        workspaces.push((id, public_id));
    }
    let state = |workspace: (i64, Uuid), actor, role: &str| AppState {
        pool: pool.clone(),
        engine: Arc::new(DeterministicEngine),
        providers: None,
        workspace_id: workspace.1,
        workspace_internal_id: Some(workspace.0),
        workspace_role: role.into(),
        actor_id: actor,
        agent_mode: "deterministic",
        steward_trigger: None,
    };
    Ok(Fixture {
        owner: state(workspaces[0], owner, "owner"),
        viewer: state(workspaces[0], viewer, "viewer"),
        foreign: state(workspaces[1], owner, "owner"),
        pool,
        admin,
    })
}

async fn project(state: &AppState, name: &str) -> Result<Uuid> {
    Ok(service::create_project(
        state,
        CreateProject {
            name: format!("[FICTIF] {name}"),
            objective: "Tester les artefacts manuels sans appel IA".into(),
        },
    )
    .await?
    .public_id)
}
fn input(title: &str) -> CreateArtifact {
    CreateArtifact {
        artifact_type: "specification".into(),
        title: title.into(),
        body_markdown: "Conserver les décisions confirmées.".into(),
        structured_content: json!({"requirements":["Historique durable"]}),
        sources: vec![],
    }
}
async fn serve(state: AppState) -> Result<(String, tokio::task::JoinHandle<()>)> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}", listener.local_addr()?);
    let auth = Arc::new(AuthRuntime::new(
        AuthMode::Local,
        state.workspace_id,
        state.actor_id,
        None,
    )?);
    let router = routes::router(Arc::new(state), auth, None, vec![]);
    Ok((
        url,
        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        }),
    ))
}
async fn scoped_tx(state: &AppState) -> Result<Transaction<'static, Postgres>> {
    let mut tx = state.pool.begin().await?;
    sqlx::query("select set_config('app.current_workspace_id',$1,true),set_config('app.current_actor_id',$2,true),set_config('app.current_workspace_role',$3,true)")
        .bind(state.workspace_internal_id.unwrap().to_string()).bind(state.actor_id.to_string()).bind(&state.workspace_role).execute(&mut *tx).await?;
    Ok(tx)
}
async fn knowledge(admin: &PgPool, project: Uuid, actor: Uuid) -> Result<Uuid> {
    let (workspace,project,node):(i64,i64,i64)=sqlx::query_as("select p.workspace_id,p.id,n.id from app.projects p join app.context_nodes n on n.project_id=p.id and n.node_key='product' where p.public_id=$1")
        .bind(project).fetch_one(admin).await?;
    let entry:i64=sqlx::query_scalar("insert into app.knowledge_entries(workspace_id,project_id,context_node_id,entry_type) values($1,$2,$3,'requirement') returning id")
        .bind(workspace).bind(project).bind(node).fetch_one(admin).await?;
    Ok(sqlx::query_scalar("insert into app.knowledge_entry_versions(knowledge_entry_id,workspace_id,project_id,context_node_id,version_number,entry_type,title,statement,author_actor_id,origin_type)
        values($1,$2,$3,$4,1,'requirement','[FICTIF] Historique','Conserver les versions validées',$5,'manual') returning public_id")
        .bind(entry).bind(workspace).bind(project).bind(node).bind(actor).fetch_one(admin).await?)
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn versions_sources_http_replay_and_tenant_isolation() -> Result<()> {
    let fixture = fixture().await?;
    let project_id = project(&fixture.owner, "Projet A").await?;
    let second_project = project(&fixture.owner, "Projet B").await?;
    let foreign_project = project(&fixture.foreign, "Autre société").await?;
    let knowledge_id = knowledge(&fixture.admin, second_project, fixture.owner.actor_id).await?;
    let foreign_knowledge =
        knowledge(&fixture.admin, foreign_project, fixture.foreign.actor_id).await?;
    let session = service::create_session(
        &fixture.owner,
        project_id,
        CreateSession {
            node_key: "product".into(),
            title: Some("[FICTIF] Cadrage".into()),
        },
    )
    .await?
    .session
    .public_id;
    let mut create = input("[FICTIF] Spécification initiale");
    create.sources = vec![
        SourceInput {
            kind: "knowledge".into(),
            public_id: knowledge_id,
        },
        SourceInput {
            kind: "session".into(),
            public_id: session,
        },
    ];
    let (url, server) = serve(fixture.owner.clone()).await?;
    let client = reqwest::Client::new();
    let command = Uuid::new_v4();
    let endpoint = format!("{url}/api/projects/{project_id}/artifacts");
    let first = client
        .post(&endpoint)
        .header("Idempotency-Key", command.to_string())
        .json(&create)
        .send()
        .await?;
    ensure!(
        first.status().is_success(),
        "create failed: {}",
        first.status()
    );
    let first: ArtifactDetail = first.json().await?;
    let replay: ArtifactDetail = client
        .post(&endpoint)
        .header("Idempotency-Key", command.to_string())
        .json(&create)
        .send()
        .await?
        .json()
        .await?;
    ensure!(
        first.artifact.public_id == replay.artifact.public_id
            && first.current_version.public_id == replay.current_version.public_id
    );
    create.title = "Changed same command".into();
    ensure!(
        client
            .post(&endpoint)
            .header("Idempotency-Key", command.to_string())
            .json(&create)
            .send()
            .await?
            .status()
            .as_u16()
            == 409
    );
    ensure!(first.current_version.sources.len() == 2);
    let origin = first
        .current_version
        .sources
        .iter()
        .find(|s| s["kind"] == "session")
        .unwrap();
    ensure!(
        origin["origin_only"] == true
            && origin["message_count"] == 0
            && origin.get("content").is_none()
    );
    ensure!(
        first
            .current_version
            .sources
            .iter()
            .any(|s| s["project_id"] == second_project.to_string())
    );
    // A conversation evolves after draft creation: validation retains its captured origin.
    sqlx::query("insert into app.messages(workspace_id,project_id,session_id,role,content)
        select workspace_id,project_id,id,'user','[FICTIF] Ajout après brouillon' from app.sessions where public_id=$1")
        .bind(session).execute(&fixture.admin).await?;
    let validated = artifacts::validate(
        &fixture.owner,
        first.artifact.public_id,
        ValidateArtifact {
            expected_version_id: first.current_version.public_id,
        },
        None,
    )
    .await?;
    ensure!(
        validated.current_version.version == 2 && validated.current_version.status == "validated"
    );
    ensure!(validated.current_version.sources == first.current_version.sources);
    ensure!(validated.current_version.content_hash == first.current_version.content_hash);
    ensure!(
        artifacts::version(
            &fixture.owner,
            first.artifact.public_id,
            first.current_version.public_id
        )
        .await?
        .status
            == "draft"
    );
    let draft = SaveDraft {
        expected_version_id: validated.current_version.public_id,
        title: "[FICTIF] Révision".into(),
        body_markdown: "Nouvelle décision".into(),
        structured_content: json!({}),
        sources: vec![],
    };
    let (a, b) = tokio::join!(
        artifacts::save_draft(
            &fixture.owner,
            first.artifact.public_id,
            draft.clone(),
            None
        ),
        artifacts::save_draft(&fixture.owner, first.artifact.public_id, draft, None)
    );
    ensure!(
        a.is_ok() != b.is_ok(),
        "Only one concurrent revision must be accepted"
    );
    ensure!(matches!(
        a.as_ref().err().or(b.as_ref().err()),
        Some(AppError::Conflict(_))
    ));
    let current = artifacts::get(&fixture.owner, first.artifact.public_id).await?;
    ensure!(current.current_version.version == 3);
    let exported = client
        .get(format!(
            "{url}/api/artifacts/{}/versions/{}/export?format=json",
            first.artifact.public_id, validated.current_version.public_id
        ))
        .send()
        .await?;
    ensure!(
        exported
            .headers()
            .get("cache-control")
            .is_some_and(|v| v == "no-store")
    );
    let exported: Value = exported.json().await?;
    ensure!(
        exported["version"]["version"] == 2
            && exported["version"]["title"] == first.current_version.title
    );
    let history = artifacts::versions(
        &fixture.owner,
        first.artifact.public_id,
        ListArtifacts {
            limit: Some(1),
            offset: Some(1),
            ..Default::default()
        },
    )
    .await?;
    ensure!(history.total == 3 && history.items[0].version == 2);
    ensure!(matches!(
        artifacts::get(&fixture.foreign, first.artifact.public_id).await,
        Err(AppError::NotFound)
    ));
    ensure!(matches!(
        artifacts::create(&fixture.viewer, project_id, input("Refus"), None).await,
        Err(AppError::Forbidden)
    ));
    ensure!(
        artifacts::get(&fixture.viewer, first.artifact.public_id)
            .await
            .is_ok()
    );
    let mut forged = input("[FICTIF] Source interdite");
    forged.sources = vec![SourceInput {
        kind: "knowledge".into(),
        public_id: foreign_knowledge,
    }];
    ensure!(matches!(
        artifacts::create(&fixture.owner, project_id, forged, None).await,
        Err(AppError::NotFound)
    ));
    let list = artifacts::list(
        &fixture.owner,
        project_id,
        ListArtifacts {
            q: Some("révision".into()),
            status: Some("draft".into()),
            ..Default::default()
        },
    )
    .await?;
    ensure!(list.total == 1 && list.items.len() == 1);
    let literal = artifacts::list(
        &fixture.owner,
        project_id,
        ListArtifacts {
            q: Some("%".into()),
            ..Default::default()
        },
    )
    .await?;
    ensure!(
        literal.total == 0,
        "Search text must not become an SQL wildcard"
    );
    let mut tx = scoped_tx(&fixture.owner).await?;
    ensure!(
        sqlx::query("update app.artifact_document_versions set title='changed' where public_id=$1")
            .bind(validated.current_version.public_id)
            .execute(&mut *tx)
            .await
            .is_err()
    );
    tx.rollback().await?;
    let mut tx = scoped_tx(&fixture.owner).await?;
    ensure!(sqlx::query("insert into app.artifact_version_sources(workspace_id,version_id,source_project_id,source_kind,knowledge_version_id,source_public_id,snapshot)
        select s.workspace_id,s.version_id,s.source_project_id,s.source_kind,s.knowledge_version_id,gen_random_uuid(),s.snapshot from app.artifact_version_sources s join app.artifact_document_versions v on v.id=s.version_id where v.public_id=$1 and s.source_kind='knowledge'")
        .bind(validated.current_version.public_id).execute(&mut *tx).await.is_err(),"Committed provenance cannot be appended");
    tx.rollback().await?;
    sqlx::query("update app.workspace_members set invitation_status='revoked' where workspace_id=$1 and actor_id=$2")
        .bind(fixture.owner.workspace_internal_id).bind(fixture.viewer.actor_id).execute(&fixture.admin).await?;
    ensure!(
        matches!(
            artifacts::get(&fixture.viewer, first.artifact.public_id).await,
            Err(AppError::NotFound)
        ),
        "Revoked context cannot read artifacts"
    );
    server.abort();
    fixture.pool.close().await;
    fixture.admin.close().await;
    Ok(())
}

#[tokio::test]
async fn destination_inheritance_cas_reset_and_no_publication() -> Result<()> {
    let fixture = fixture().await?;
    let project_id = project(&fixture.owner, "Destination").await?;
    let baseline = artifacts::destinations(&fixture.owner, Some(project_id)).await?;
    ensure!(
        baseline
            .items
            .iter()
            .all(|s| s.origin == "default" && s.provider == "internal" && s.revision == 0)
    );
    let company = SetDestination {
        artifact_type: "specification".into(),
        provider: "notion".into(),
        target_id: Some(Uuid::new_v4().to_string()),
        label: "[FICTIF] Documentation".into(),
        expected_revision: 0,
    };
    artifacts::set_destination(&fixture.owner, None, company.clone(), None).await?;
    ensure!(matches!(
        artifacts::set_destination(&fixture.owner, None, company, None).await,
        Err(AppError::Conflict(_))
    ));
    let inherited = artifacts::destinations(&fixture.owner, Some(project_id)).await?;
    let inherited = inherited
        .items
        .iter()
        .find(|s| s.artifact_type == "specification")
        .unwrap();
    ensure!(
        inherited.origin == "company" && inherited.provider == "notion" && inherited.revision == 0
    );
    let local = SetDestination {
        artifact_type: "specification".into(),
        provider: "internal".into(),
        target_id: None,
        label: String::new(),
        expected_revision: 0,
    };
    artifacts::set_destination(&fixture.owner, Some(project_id), local.clone(), None).await?;
    let reset = artifacts::reset_destination(
        &fixture.owner,
        Some(project_id),
        ResetDestination {
            artifact_type: "specification".into(),
            expected_revision: 1,
        },
        None,
    )
    .await?;
    let reset = reset
        .items
        .iter()
        .find(|s| s.artifact_type == "specification")
        .unwrap();
    ensure!(reset.origin == "company" && reset.provider == "notion" && reset.revision == 2);
    ensure!(
        matches!(
            artifacts::set_destination(&fixture.owner, Some(project_id), local, None).await,
            Err(AppError::Conflict(_))
        ),
        "Reset must not erase the CAS revision"
    );
    let own = artifacts::destinations(&fixture.foreign, None).await?;
    ensure!(own.items.iter().all(|s| s.origin == "default"));
    let empty = CreateArtifact {
        body_markdown: String::new(),
        structured_content: json!({}),
        ..input("[FICTIF] Vide")
    };
    let empty = artifacts::create(&fixture.owner, project_id, empty, None).await?;
    ensure!(matches!(
        artifacts::validate(
            &fixture.owner,
            empty.artifact.public_id,
            ValidateArtifact {
                expected_version_id: empty.current_version.public_id
            },
            None
        )
        .await,
        Err(AppError::Invalid(_))
    ));
    ensure!(
        empty.current_version.status == "draft",
        "Destination configuration does not publish a version"
    );
    fixture.pool.close().await;
    fixture.admin.close().await;
    Ok(())
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn agent_draft_has_typed_content_exact_artifact_sources_and_http_replay() -> Result<()> {
    use ai_center_server::artifacts::generation_contract::GenerateArtifact;
    let f = fixture().await?;
    let project_id = project(&f.owner, "Génération").await?;
    let session = service::create_session(
        &f.owner,
        project_id,
        CreateSession {
            node_key: "product".into(),
            title: Some("[FICTIF] Préparation".into()),
        },
    )
    .await?;
    let manual = artifacts::create(
        &f.owner,
        project_id,
        input("[FICTIF] Règles antérieures"),
        None,
    )
    .await?;
    let source = artifacts::validate(
        &f.owner,
        manual.artifact.public_id,
        ValidateArtifact {
            expected_version_id: manual.current_version.public_id,
        },
        None,
    )
    .await?;
    let request = GenerateArtifact {
        session_id: session.session.public_id,
        artifact_type: "product_tickets".into(),
        instructions: "[FICTIF] Décomposer la conservation des décisions en tickets".into(),
    };
    let (url, server) = serve(f.owner.clone()).await?;
    let client = reqwest::Client::new();
    let key = Uuid::new_v4();
    let endpoint = format!("{url}/api/projects/{project_id}/artifacts/generate");
    let response = client
        .post(&endpoint)
        .header("Idempotency-Key", key.to_string())
        .json(&request)
        .send()
        .await?;
    ensure!(
        response.status().is_success(),
        "generation rejected: {}",
        response.status()
    );
    let draft: ArtifactDetail = response.json().await?;
    ensure!(
        draft.current_version.status == "draft" && draft.current_version.validated_at.is_none()
    );
    ensure!(
        draft.current_version.structured_content["draft"]["tickets"]
            .as_array()
            .unwrap()
            .len()
            == 1
    );
    ensure!(
        draft
            .current_version
            .sources
            .iter()
            .any(|s| s["kind"] == "artifact_version"
                && s["public_id"] == source.current_version.public_id.to_string()
                && s["content_hash"] == source.current_version.content_hash)
    );
    let replay: ArtifactDetail = client
        .post(&endpoint)
        .header("Idempotency-Key", key.to_string())
        .json(&request)
        .send()
        .await?
        .json()
        .await?;
    ensure!(replay.artifact.public_id == draft.artifact.public_id);
    let receipt: Value = client
        .get(format!(
            "{url}/api/projects/{project_id}/artifact-generations/{key}"
        ))
        .send()
        .await?
        .json()
        .await?;
    ensure!(
        receipt["status"] == "completed"
            && receipt["result"]["artifact"]["public_id"] == draft.artifact.public_id.to_string()
    );
    let colleague_receipt =
        service::artifact_generation::receipt(&f.viewer, project_id, key).await?;
    ensure!(colleague_receipt["result"].is_null() && colleague_receipt["can_retry"] == false);
    ensure!(matches!(
        service::artifact_generation::receipt(&f.foreign, project_id, key).await,
        Err(AppError::NotFound)
    ));

    // Editing after later conversation messages retains the evidence seen by
    // the generation, not the conversation's newer head.
    sqlx::query("insert into app.messages(workspace_id,project_id,session_id,role,content) select workspace_id,project_id,id,'user','[FICTIF] Message postérieur' from app.sessions where public_id=$1")
        .bind(session.session.public_id).execute(&f.admin).await?;
    let mut edited_content = draft.current_version.structured_content.clone();
    edited_content["draft"]["title"] = json!("[FICTIF] Titre relu");
    let edited = artifacts::save_draft(
        &f.owner,
        draft.artifact.public_id,
        SaveDraft {
            expected_version_id: draft.current_version.public_id,
            title: "[FICTIF] Titre relu".into(),
            body_markdown: draft.current_version.body_markdown.clone(),
            structured_content: edited_content,
            sources: draft
                .current_version
                .sources
                .iter()
                .map(|source| SourceInput {
                    kind: source["kind"].as_str().unwrap().into(),
                    public_id: source["public_id"].as_str().unwrap().parse().unwrap(),
                })
                .collect(),
        },
        None,
    )
    .await?;
    ensure!(
        edited.current_version.sources == draft.current_version.sources,
        "Editing must retain exact source snapshots"
    );
    sqlx::query("update app.idempotency_records set created_at=now()-interval '2 days',expires_at=now()-interval '1 second' where idempotency_key=$1 and operation_key='artifact.generate'")
        .bind(key.to_string()).execute(&f.admin).await?;
    ensure!(
        client
            .post(&endpoint)
            .header("Idempotency-Key", key.to_string())
            .json(&request)
            .send()
            .await?
            .status()
            == reqwest::StatusCode::CONFLICT
    );
    let expired_receipt = service::artifact_generation::receipt(&f.owner, project_id, key).await?;
    ensure!(
        expired_receipt["result"]["artifact"]["public_id"] == draft.artifact.public_id.to_string()
    );
    let runs:i64=sqlx::query_scalar("select count(*) from app.model_runs r join app.projects p on p.id=r.project_id where p.public_id=$1 and operation='generate_artifact' and r.status='completed'").bind(project_id).fetch_one(&f.admin).await?;
    ensure!(runs == 1, "replay must not call the engine again");
    let graph = ai_center_server::company::graph(
        &f.owner,
        ai_center_server::company::models::GraphQuery {
            project_id: Some(project_id),
            limit: None,
        },
    )
    .await?;
    ensure!(graph.edges.iter().any(|edge| edge.source_public_id
        == draft.current_version.public_id
        && edge.target_public_id == source.current_version.public_id));
    ensure!(matches!(
        service::artifact_generation::generate(&f.viewer, project_id, request.clone(), None).await,
        Err(AppError::Forbidden)
    ));
    ensure!(matches!(
        service::artifact_generation::generate(&f.foreign, project_id, request.clone(), None).await,
        Err(AppError::NotFound)
    ));
    let unsupported = GenerateArtifact {
        artifact_type: "technical_tickets".into(),
        ..request
    };
    ensure!(matches!(
        service::artifact_generation::generate(&f.owner, project_id, unsupported, None).await,
        Err(AppError::Invalid(_))
    ));
    let publication_count:i64=sqlx::query_scalar("select count(*) from app.publication_jobs j join app.projects p on p.id=j.project_id where p.public_id=$1").bind(project_id).fetch_one(&f.admin).await?;
    ensure!(publication_count == 0);
    server.abort();
    f.pool.close().await;
    f.admin.close().await;
    Ok(())
}

#[tokio::test]
async fn conversion_preserves_exact_historical_deliverable_and_requires_review() -> Result<()> {
    let f = fixture().await?;
    let project_id = project(&f.owner, "Conversion historique").await?;
    let other = project(&f.owner, "Autre projet").await?;
    let original = json!({"objective":"[FICTIF] Objectif v1","requirements":["Conserver l’historique"],"unknowns":["Échéance à définir"]});
    let id:Uuid=sqlx::query_scalar("insert into app.deliverables(workspace_id,project_id,context_node_id,contract_id,deliverable_type,title,summary,content,status,coverage_status,version,source_graph_version,content_hash) select p.workspace_id,p.id,n.id,c.id,'feature-brief','[FICTIF] Ancienne spécification','[FICTIF] Version conservée',$2,'superseded','missing',1,p.graph_version,$3 from app.projects p join app.context_nodes n on n.project_id=p.id and n.node_key='product' join app.deliverable_contracts c on c.template_id=p.template_id and c.contract_key='feature-brief' where p.public_id=$1 returning public_id")
        .bind(project_id).bind(&original).bind("a".repeat(64)).fetch_one(&f.admin).await?;
    let draft = artifacts::conversion::convert(&f.owner, project_id, id, None).await?;
    ensure!(
        draft.artifact.artifact_type == "specification" && draft.current_version.status == "draft"
    );
    ensure!(draft.current_version.structured_content["content"] == original);
    ensure!(
        draft.current_version.structured_content["origin"]["status_at_capture"] == "superseded"
    );
    ensure!(draft.current_version.sources[0]["public_id"] == id.to_string());
    ensure!(draft.current_version.sources[0]["version"] == 1);
    ensure!(matches!(
        artifacts::conversion::convert(&f.owner, other, id, None).await,
        Err(AppError::NotFound)
    ));
    ensure!(matches!(
        artifacts::conversion::convert(&f.foreign, project_id, id, None).await,
        Err(AppError::NotFound)
    ));
    ensure!(matches!(
        artifacts::conversion::convert(&f.viewer, project_id, id, None).await,
        Err(AppError::Forbidden)
    ));
    f.pool.close().await;
    f.admin.close().await;
    Ok(())
}

struct ArchiveDuringDraft {
    admin: PgPool,
    project: Uuid,
    output: Value,
}
#[async_trait::async_trait]
impl ai_center_server::integrations::providers::StructuredTransport for ArchiveDuringDraft {
    fn provider_name(&self) -> &'static str {
        "fixture-archive"
    }
    async fn generate(
        &self,
        _model: &str,
        _operation: &str,
        _instructions: &str,
        _input: &Value,
        _schema: &Value,
    ) -> ai_center_server::error::AppResult<
        ai_center_server::integrations::providers::StructuredResponse,
    > {
        sqlx::query("update app.projects set status='archived' where public_id=$1")
            .bind(self.project)
            .execute(&self.admin)
            .await?;
        Ok(
            ai_center_server::integrations::providers::StructuredResponse {
                output: self.output.clone(),
                metadata: ai_center_server::agent::AgentRunMetadata::default(),
            },
        )
    }
    async fn list_models(
        &self,
    ) -> ai_center_server::error::AppResult<
        Vec<ai_center_server::integrations::providers::ProviderModel>,
    > {
        Ok(vec![])
    }
}
#[tokio::test]
async fn archive_during_generation_closes_model_run_without_saving_a_draft() -> Result<()> {
    use artifacts::generation_contract::{
        ArtifactGenerationInput, GenerateArtifact, deterministic,
    };
    let f = fixture().await?;
    let project_id = project(&f.owner, "Archivage pendant génération").await?;
    let session = service::create_session(
        &f.owner,
        project_id,
        CreateSession {
            node_key: "product".into(),
            title: None,
        },
    )
    .await?;
    let draft = deterministic(ArtifactGenerationInput {
        artifact_type: "kickoff".into(),
        instructions: "[FICTIF] Cadrer".into(),
        agent_instructions: String::new(),
        objective: "[FICTIF] Archive".into(),
        context: json!({}),
        conversation: json!({}),
        source_ids: vec![],
    })?
    .output;
    let mut state = f.owner.clone();
    state.engine = Arc::new(ai_center_server::agent::StructuredEngine::with_transport(
        Arc::new(ArchiveDuringDraft {
            admin: f.admin.clone(),
            project: project_id,
            output: json!(draft),
        }),
        "fixture".into(),
    ));
    ensure!(
        service::artifact_generation::generate(
            &state,
            project_id,
            GenerateArtifact {
                session_id: session.session.public_id,
                artifact_type: "kickoff".into(),
                instructions: "[FICTIF] Cadrer".into()
            },
            None
        )
        .await
        .is_err()
    );
    let states:Vec<String>=sqlx::query_scalar("select r.status from app.model_runs r join app.projects p on p.id=r.project_id where p.public_id=$1 and operation='generate_artifact'").bind(project_id).fetch_all(&f.admin).await?;
    ensure!(
        states == vec!["failed"],
        "A rejected finalization must close the model attempt"
    );
    let count:i64=sqlx::query_scalar("select count(*) from app.artifact_documents d join app.projects p on p.id=d.project_id where p.public_id=$1").bind(project_id).fetch_one(&f.admin).await?;
    ensure!(count == 0);
    f.pool.close().await;
    f.admin.close().await;
    Ok(())
}
