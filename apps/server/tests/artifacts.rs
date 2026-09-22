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
