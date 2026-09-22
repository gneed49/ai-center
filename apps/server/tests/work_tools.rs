//! Fictitious data and local HTTP only. Run with the isolated integration stack.
#![allow(clippy::too_many_lines)]
use ai_center_server::{
    agent::DeterministicEngine,
    artifacts::{self, CreateArtifact, SetDestination, ValidateArtifact},
    auth::RequestContext,
    error::AppError,
    models::CreateProject,
    providers::{ProviderRuntime, encryption::CredentialCipher},
    service::{self, AppState},
    work_tools::{
        self, DisableConnection, PublishArtifact, SaveConnection, client::ToolClient, worker,
    },
};
use anyhow::{Result, ensure};
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::any};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use secrecy::SecretString;
use serde_json::{Value, json};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{
    fmt::Write as _,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use uuid::Uuid;

struct Fixture {
    owner: AppState,
    editor: AppState,
    viewer: AppState,
    foreign: AppState,
    admin: PgPool,
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
    ensure!(!bypass, "Tests require an unprivileged runtime");
    let actors = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
    let mut workspaces = Vec::new();
    for _ in 0..2 {
        let (id,public_id):(i64,Uuid)=sqlx::query_as("insert into app.workspaces(owner_actor_id,name) values($1,'[FICTIF] Publications') returning id,public_id").bind(actors[0]).fetch_one(&admin).await?;
        for (actor, role) in actors.iter().zip(["owner", "editor", "viewer"]) {
            sqlx::query("insert into app.workspace_members(workspace_id,actor_id,role,invitation_status,accepted_at) values($1,$2,$3,'accepted',now())").bind(id).bind(actor).bind(role).execute(&admin).await?;
        }
        workspaces.push((id, public_id));
    }
    let owner = AppState {
        pool,
        engine: Arc::new(DeterministicEngine),
        providers: Some(Arc::new(ProviderRuntime {
            cipher: Some(CredentialCipher::from_bytes(&[37; 32])?),
            subscriptions: None,
        })),
        workspace_id: workspaces[0].1,
        workspace_internal_id: Some(workspaces[0].0),
        workspace_role: "owner".into(),
        actor_id: actors[0],
        agent_mode: "deterministic",
        steward_trigger: None,
    };
    let scope = |actor, role: &str, ws: usize| {
        owner.scoped(&RequestContext {
            actor_id: actor,
            workspace_id: workspaces[ws].1,
            workspace_internal_id: Some(workspaces[ws].0),
            workspace_role: role.into(),
        })
    };
    Ok(Fixture {
        editor: scope(actors[1], "editor", 0),
        viewer: scope(actors[2], "viewer", 0),
        foreign: scope(actors[0], "owner", 1),
        owner,
        admin,
    })
}
fn connection(id: Uuid) -> SaveConnection {
    SaveConnection {
        id,
        provider: "github".into(),
        name: "[FICTIF] GitHub".into(),
        expected_revision: 0,
        api_key: Some(SecretString::from("synthetic-credential-no-real-account")),
    }
}
async fn create_artifact(state: &AppState) -> Result<(Uuid, Uuid)> {
    let project = service::create_project(
        state,
        CreateProject {
            name: "[FICTIF] Projet".into(),
            objective: "Publication locale de test".into(),
        },
    )
    .await?;
    let created = artifacts::create(
        state,
        project.public_id,
        CreateArtifact {
            artifact_type: "specification".into(),
            title: "[FICTIF] Specification".into(),
            body_markdown: "Exigence confirmée par une personne.".into(),
            structured_content: json!({}),
            sources: vec![],
        },
        None,
    )
    .await?;
    let validated = artifacts::validate(
        state,
        created.artifact.public_id,
        ValidateArtifact {
            expected_version_id: created.current_version.public_id,
        },
        None,
    )
    .await?;
    Ok((
        validated.artifact.public_id,
        validated.current_version.public_id,
    ))
}
async fn destination(state: &AppState) -> Result<()> {
    artifacts::set_destination(
        state,
        None,
        SetDestination {
            artifact_type: "specification".into(),
            provider: "github".into(),
            target_id: Some("fictif/repository".into()),
            label: "[FICTIF] Repository".into(),
            expected_revision: 0,
        },
        None,
    )
    .await?;
    Ok(())
}
fn publish_input(version_id: Uuid, connection_id: Uuid) -> PublishArtifact {
    PublishArtifact {
        version_id,
        connection_id,
        expected_provider: "github".into(),
        expected_target_id: "fictif/repository".into(),
    }
}
#[derive(Clone, Default)]
struct Remote {
    calls: Arc<AtomicUsize>,
    body: Arc<Mutex<Value>>,
    mode: Arc<Mutex<&'static str>>,
    started: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
}
async fn remote_handler(
    State(remote): State<Remote>,
    request: axum::http::Request<axum::body::Body>,
) -> axum::response::Response {
    remote.calls.fetch_add(1, Ordering::SeqCst);
    let method = request.method().clone();
    if method == axum::http::Method::POST {
        let bytes = axum::body::to_bytes(request.into_body(), 100_000)
            .await
            .expect("body");
        *remote.body.lock().expect("body") = serde_json::from_slice(&bytes).expect("json");
    }
    let mode = *remote.mode.lock().expect("mode");
    if mode == "timeout" {
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    if mode == "hold" {
        remote.started.notify_one();
        remote.release.notified().await;
    }
    if mode == "unavailable" {
        return StatusCode::FORBIDDEN.into_response();
    }
    let body = remote.body.lock().expect("body").clone();
    Json(json!({"number":42,"html_url":"https://github.com/fictif/repository/issues/42","title":body["title"],"body":body["body"],"updated_at":"2026-09-21T12:00:00Z"})).into_response()
}
async fn remote_server() -> Result<(ToolClient, Remote, tokio::task::JoinHandle<()>)> {
    remote_server_with_timeout(Duration::from_millis(80)).await
}
async fn remote_server_with_timeout(
    timeout: Duration,
) -> Result<(ToolClient, Remote, tokio::task::JoinHandle<()>)> {
    let remote = Remote::default();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let client = ToolClient::loopback(&format!("http://{}/", listener.local_addr()?), timeout)?;
    let app = Router::new()
        .fallback(any(remote_handler))
        .with_state(remote.clone());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    Ok((client, remote, task))
}

#[tokio::test]
async fn in_flight_publication_detects_demotion_and_a_brief_pause_resume() -> Result<()> {
    for demote in [true, false] {
        let f = fixture().await?;
        let id = Uuid::new_v4();
        work_tools::save_connection(&f.owner, connection(id), None).await?;
        destination(&f.owner).await?;
        let (artifact, version) = create_artifact(&f.owner).await?;
        let job =
            work_tools::publish(&f.editor, artifact, publish_input(version, id), None).await?;
        let (client, remote, http) = remote_server_with_timeout(Duration::from_secs(10)).await?;
        *remote.mode.lock().unwrap() = "hold";
        let worker_state = f.owner.clone();
        let task = tokio::spawn(async move { worker::drain_one(&worker_state, &client).await });
        tokio::time::timeout(Duration::from_secs(5), remote.started.notified()).await?;
        if demote {
            sqlx::query("update app.workspace_members set role='viewer' where workspace_id=$1 and actor_id=$2")
                .bind(f.editor.workspace_internal_id).bind(f.editor.actor_id).execute(&f.admin).await?;
        } else {
            for (enabled, expected_generation) in [(false, 0), (true, 1)] {
                ai_center_server::automation::set(
                    &f.owner,
                    ai_center_server::automation::SetControl {
                        enabled,
                        expected_generation,
                    },
                    None,
                )
                .await?;
            }
        }
        assert!(tokio::time::timeout(Duration::from_secs(5), task).await???);
        let detail = work_tools::detail(&f.owner, job.public_id).await?;
        assert_eq!(detail.publication.status, "needs_review");
        assert_eq!(
            detail.publication.error_code.as_deref(),
            Some("automation_or_permissions_changed_during_request")
        );
        assert!(detail.publication.external_id.is_none());
        assert_eq!(remote.calls.load(Ordering::SeqCst), 1);
        remote.release.notify_one();
        http.abort();
        f.owner.pool.close().await;
        f.admin.close().await;
    }
    Ok(())
}

#[tokio::test]
async fn credentials_roles_publication_deduplication_and_canonical_observations() -> Result<()> {
    let f = fixture().await?;
    let id = Uuid::new_v4();
    assert!(matches!(
        work_tools::save_connection(&f.editor, connection(id), None).await,
        Err(AppError::Forbidden)
    ));
    let saved = work_tools::save_connection(&f.owner, connection(id), None).await?;
    let output = serde_json::to_string(&saved)?;
    assert!(!output.contains("credential"));
    let encrypted: Vec<u8> = sqlx::query_scalar(
        "select encrypted_credential from app.work_tool_connections where public_id=$1",
    )
    .bind(id)
    .fetch_one(&f.admin)
    .await?;
    assert!(!encrypted.windows(10).any(|bytes| bytes == b"synthetic-"));
    assert!(
        work_tools::settings(&f.foreign)
            .await?
            .connections
            .is_empty()
    );
    assert!(
        work_tools::save_connection(&f.owner, connection(id), None)
            .await
            .is_err()
    );
    destination(&f.owner).await?;
    let (artifact, version) = create_artifact(&f.owner).await?;
    assert!(matches!(
        work_tools::publish(&f.viewer, artifact, publish_input(version, id), None).await,
        Err(AppError::Forbidden)
    ));
    let a = work_tools::publish(&f.editor, artifact, publish_input(version, id), None);
    let b = work_tools::publish(&f.editor, artifact, publish_input(version, id), None);
    let (a, b) = tokio::join!(a, b);
    let a = a?;
    assert_eq!(a.public_id, b?.public_id);
    assert!(work_tools::detail(&f.foreign, a.public_id).await.is_err());
    let (client, remote, task) = remote_server().await?;
    assert!(worker::drain_one(&f.owner, &client).await?);
    assert!(!worker::drain_one(&f.owner, &client).await?);
    assert_eq!(remote.calls.load(Ordering::SeqCst), 1);
    let detail = work_tools::detail(&f.viewer, a.public_id).await?;
    assert_eq!(detail.publication.status, "succeeded");
    assert_eq!(detail.observations.len(), 1);
    assert_eq!(detail.observations[0].snapshot["complete"], true);
    assert_eq!(
        work_tools::observe_with_client(&f.editor, a.public_id, None, &client)
            .await?
            .status,
        "succeeded"
    );
    remote.body.lock().expect("body")["body"] = json!(format!(
        "External edit\nAI Center publication: {}",
        a.public_id
    ));
    assert_eq!(
        work_tools::observe_with_client(&f.editor, a.public_id, None, &client)
            .await?
            .status,
        "conflict"
    );
    // Repeated reads never silently acknowledge an external edit.
    assert_eq!(
        work_tools::observe_with_client(&f.editor, a.public_id, None, &client)
            .await?
            .status,
        "conflict"
    );
    *remote.mode.lock().expect("mode") = "unavailable";
    assert_eq!(
        work_tools::observe_with_client(&f.editor, a.public_id, None, &client)
            .await?
            .status,
        "unavailable"
    );
    let detail = work_tools::detail(&f.owner, a.public_id).await?;
    assert_eq!(detail.publication.external_id.as_deref(), Some("42"));
    assert_eq!(detail.observations.len(), 5);
    let changed =
        sqlx::query("update app.publication_jobs set external_id='43' where public_id=$1")
            .bind(a.public_id)
            .execute(&f.admin)
            .await;
    assert!(
        changed.is_err(),
        "A receipt identity is immutable even for the administrative test role"
    );
    task.abort();
    Ok(())
}

#[tokio::test]
async fn ambiguous_create_reconciles_without_duplicate_and_expired_lease_never_requeues()
-> Result<()> {
    let f = fixture().await?;
    let id = Uuid::new_v4();
    work_tools::save_connection(&f.owner, connection(id), None).await?;
    destination(&f.owner).await?;
    let (artifact, version) = create_artifact(&f.owner).await?;
    let job = work_tools::publish(&f.editor, artifact, publish_input(version, id), None).await?;
    let (client, remote, task) = remote_server().await?;
    *remote.mode.lock().expect("mode") = "timeout";
    worker::drain_one(&f.owner, &client).await?;
    assert_eq!(
        work_tools::detail(&f.owner, job.public_id)
            .await?
            .publication
            .status,
        "needs_review"
    );
    assert!(!worker::drain_one(&f.owner, &client).await?);
    assert_eq!(remote.calls.load(Ordering::SeqCst), 1);
    *remote.mode.lock().expect("mode") = "success";
    let reconciled =
        work_tools::observe_with_client(&f.editor, job.public_id, Some("42".into()), &client)
            .await?;
    assert_eq!(reconciled.status, "succeeded");
    assert_eq!(remote.calls.load(Ordering::SeqCst), 2);
    let (artifact2, version2) = create_artifact(&f.owner).await?;
    let job2 = work_tools::publish(&f.editor, artifact2, publish_input(version2, id), None).await?;
    let claimed: Option<(Uuid,)> = sqlx::query_as("select job_id from app.claim_publication_job()")
        .fetch_optional(&f.owner.pool)
        .await?;
    assert_eq!(claimed.expect("claim").0, job2.public_id);
    sqlx::query(
        "update app.publication_jobs set lease_until=now()-interval '1 minute' where public_id=$1",
    )
    .bind(job2.public_id)
    .execute(&f.admin)
    .await?;
    assert!(!worker::drain_one(&f.owner, &client).await?);
    assert_eq!(
        work_tools::detail(&f.owner, job2.public_id)
            .await?
            .publication
            .status,
        "needs_review"
    );
    assert_eq!(remote.calls.load(Ordering::SeqCst), 2);
    task.abort();
    Ok(())
}

#[tokio::test]
async fn revoked_actor_or_rotated_connection_cancels_before_http() -> Result<()> {
    let f = fixture().await?;
    let id = Uuid::new_v4();
    work_tools::save_connection(&f.owner, connection(id), None).await?;
    destination(&f.owner).await?;
    let (artifact, version) = create_artifact(&f.owner).await?;
    let job = work_tools::publish(&f.editor, artifact, publish_input(version, id), None).await?;
    work_tools::disable_connection(
        &f.owner,
        id,
        DisableConnection {
            expected_revision: 1,
        },
        None,
    )
    .await?;
    let (client, remote, task) = remote_server().await?;
    worker::drain_one(&f.owner, &client).await?;
    assert_eq!(
        work_tools::detail(&f.owner, job.public_id)
            .await?
            .publication
            .status,
        "cancelled"
    );
    let mut input = connection(id);
    input.expected_revision = 2;
    work_tools::save_connection(&f.owner, input, None).await?;
    let (artifact2, version2) = create_artifact(&f.owner).await?;
    let job2 = work_tools::publish(&f.editor, artifact2, publish_input(version2, id), None).await?;
    sqlx::query("update app.workspace_members set invitation_status='revoked' where workspace_id=$1 and actor_id=$2").bind(f.editor.workspace_internal_id).bind(f.editor.actor_id).execute(&f.admin).await?;
    worker::drain_one(&f.owner, &client).await?;
    assert_eq!(
        work_tools::detail(&f.owner, job2.public_id)
            .await?
            .publication
            .status,
        "cancelled"
    );
    assert_eq!(remote.calls.load(Ordering::SeqCst), 0);
    task.abort();
    Ok(())
}

#[tokio::test]
async fn company_quotas_and_queued_cancellation_prevent_remote_side_effects() -> Result<()> {
    let f = fixture().await?;
    let connection_id = Uuid::new_v4();
    work_tools::save_connection(&f.owner, connection(connection_id), None).await?;
    destination(&f.owner).await?;
    let (artifact, version) = create_artifact(&f.owner).await?;
    let first = work_tools::publish(
        &f.editor,
        artifact,
        publish_input(version, connection_id),
        None,
    )
    .await?;
    let limits = work_tools::reliability::limits()?;
    // Saturate only this fictitious company's queue without touching a provider.
    sqlx::query("insert into app.publication_jobs(workspace_id,project_id,artifact_version_id,connection_id,connection_revision,requested_by_actor_id,provider,target_id,title,body_markdown,content_hash) select j.workspace_id,j.project_id,j.artifact_version_id,j.connection_id,j.connection_revision,j.requested_by_actor_id,j.provider,'fictif/queued-'||n::text,j.title,j.body_markdown,j.content_hash from app.publication_jobs j cross join generate_series(1,$2::integer) n where j.public_id=$1")
        .bind(first.public_id).bind(i32::try_from(limits.max_pending_publications-1)?).execute(&f.admin).await?;
    let duplicate = work_tools::publish(
        &f.editor,
        artifact,
        publish_input(version, connection_id),
        None,
    )
    .await?;
    assert_eq!(duplicate.public_id, first.public_id);
    let (second_artifact, second_version) = create_artifact(&f.owner).await?;
    assert!(matches!(
        work_tools::publish(
            &f.editor,
            second_artifact,
            publish_input(second_version, connection_id),
            None
        )
        .await,
        Err(AppError::Capacity { .. })
    ));
    assert!(matches!(
        work_tools::cancel(&f.viewer, first.public_id, None).await,
        Err(AppError::Forbidden)
    ));
    assert_eq!(
        work_tools::cancel(&f.editor, first.public_id, None)
            .await?
            .status,
        "cancelled"
    );
    let second = work_tools::publish(
        &f.editor,
        second_artifact,
        publish_input(second_version, connection_id),
        None,
    )
    .await?;
    let settings = work_tools::settings(&f.owner).await?;
    assert_eq!(settings.usage.queued, limits.max_pending_publications);
    assert_eq!(settings.usage.known_cost_usd, None);
    let jobs: Vec<Uuid> = sqlx::query_scalar(
        "select public_id from app.publication_jobs where workspace_id=$1 and status='queued'",
    )
    .bind(f.owner.workspace_internal_id)
    .fetch_all(&f.admin)
    .await?;
    for job in jobs {
        work_tools::cancel(&f.owner, job, None).await?;
    }
    let (client, remote, task) = remote_server().await?;
    assert!(!worker::drain_one(&f.owner, &client).await?);
    assert_eq!(remote.calls.load(Ordering::SeqCst), 0);
    // A persisted admission ledger bounds reads across application replicas.
    sqlx::query("insert into app.audit_events(workspace_id,actor_id,action,object_kind,object_public_id,after_state) select $1,$2,'work_tool.remote_read.admitted','work_tool',$3,'{}'::jsonb from generate_series(1,$4::integer)")
        .bind(f.owner.workspace_internal_id).bind(f.owner.actor_id).bind(second.public_id).bind(i32::try_from(limits.max_remote_reads_per_hour)?).execute(&f.admin).await?;
    sqlx::query(
        "update app.publication_jobs set status='needs_review',attempt_count=1 where public_id=$1",
    )
    .bind(second.public_id)
    .execute(&f.admin)
    .await?;
    assert!(matches!(
        work_tools::observe_with_client(&f.owner, second.public_id, Some("42".into()), &client)
            .await,
        Err(AppError::Capacity { .. })
    ));
    assert_eq!(remote.calls.load(Ordering::SeqCst), 0);
    task.abort();
    Ok(())
}

#[tokio::test]
async fn github_code_corpus_keeps_exact_bytes_scopes_and_insufficient_evidence() -> Result<()> {
    use work_tools::code::{self, ReadCode};
    const COMMIT: &str = "1111111111111111111111111111111111111111";
    const TREE: &str = "2222222222222222222222222222222222222222";
    const CONTENT: &str = "// [FICTIF] Code read as data only.\nfn enabled() -> bool { true }\n";
    let mut sha = ring::digest::Context::new(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY);
    sha.update(format!("blob {}\0", CONTENT.len()).as_bytes());
    sha.update(CONTENT.as_bytes());
    let blob = sha
        .finish()
        .as_ref()
        .iter()
        .fold(String::with_capacity(40), |mut output, byte| {
            let _ = write!(output, "{byte:02x}");
            output
        });
    let blob_for_server = blob.clone();
    let app=Router::new().fallback(axum::routing::get(move |uri:axum::http::Uri|{
        let blob=blob_for_server.clone();async move{
            Json(if uri.path().ends_with(&format!("/commits/{COMMIT}")){json!({"sha":COMMIT,"tree":{"sha":TREE}})}
            else if uri.path().ends_with(&format!("/trees/{TREE}")){json!({"sha":TREE,"truncated":false,"tree":[{"path":"app.rs","type":"blob","mode":"100644","sha":blob,"size":CONTENT.len()}]})}
            else{json!({"sha":blob,"encoding":"base64","size":CONTENT.len(),"content":STANDARD.encode(CONTENT)})})
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let client = ToolClient::loopback(
        &format!("http://{}/", listener.local_addr()?),
        Duration::from_secs(1),
    )?;
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });
    let f = fixture().await?;
    let connection_id = Uuid::new_v4();
    work_tools::save_connection(&f.owner, connection(connection_id), None).await?;
    let project = service::create_project(
        &f.owner,
        CreateProject {
            name: "[FICTIF] Code scope".into(),
            objective: "Read selected files, no execution".into(),
        },
    )
    .await?;
    let input = || ReadCode {
        connection_id,
        repository: "fictif/repository".into(),
        commit_sha: COMMIT.into(),
        paths: vec!["app.rs".into(), "missing.rs".into()],
    };
    assert!(matches!(
        code::read_using(&f.viewer, project.public_id, input(), None, &client).await,
        Err(AppError::Forbidden)
    ));
    let corpus = code::read_using(&f.editor, project.public_id, input(), None, &client).await?;
    assert!(corpus.corpus.commit_verified);
    assert_eq!(corpus.files.len(), 2);
    let file = corpus
        .files
        .iter()
        .find(|file| file.path == "app.rs")
        .expect("file");
    assert_eq!(file.status, "code_read");
    assert_eq!(file.content_text.as_deref(), Some(CONTENT));
    assert_eq!(file.blob_sha.as_deref(), Some(blob.as_str()));
    assert_eq!(file.line_count, 2);
    let missing = corpus
        .files
        .iter()
        .find(|file| file.path == "missing.rs")
        .expect("missing");
    assert_eq!(missing.status, "missing");
    assert!(missing.content_text.is_none());
    let graph = ai_center_server::company::graph(
        &f.viewer,
        ai_center_server::company::models::GraphQuery {
            project_id: Some(project.public_id),
            limit: Some(100),
        },
    )
    .await?;
    for observed in [&file, &missing] {
        let node = graph
            .nodes
            .iter()
            .find(|node| node.id == observed.public_id)
            .expect("observed file in graph");
        assert_eq!(
            node.app_path.as_deref(),
            Some(
                format!(
                    "/projects/{}/code?observation={}&file={}",
                    project.public_id, corpus.corpus.public_id, observed.public_id,
                )
                .as_str()
            )
        );
    }
    assert!(
        code::detail(&f.foreign, corpus.corpus.public_id)
            .await
            .is_err()
    );
    assert_eq!(
        code::detail(&f.viewer, corpus.corpus.public_id)
            .await?
            .files[0]
            .content_text
            .as_deref(),
        Some(CONTENT)
    );
    let frozen = sqlx::query(
        "update app.github_code_file_observations set content_text='changed' where public_id=$1",
    )
    .bind(file.public_id)
    .execute(&f.admin)
    .await;
    assert!(frozen.is_err());
    assert_eq!(
        code::list(&f.viewer, project.public_id, code::CodeQuery::default())
            .await?
            .items
            .len(),
        1
    );
    // A later read produces a new immutable observation; the first remains exact.
    let newer = code::read_using(&f.editor, project.public_id, input(), None, &client).await?;
    assert_ne!(newer.corpus.public_id, corpus.corpus.public_id);
    assert_eq!(
        code::detail(&f.owner, corpus.corpus.public_id).await?.files[0]
            .content_text
            .as_deref(),
        Some(CONTENT)
    );
    task.abort();
    Ok(())
}
