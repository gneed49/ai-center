//! [FICTIF] Real production routes/RLS/browser; only provider HTTP is loopback.
use super::browser::{Tasks, block_official_reads, node};
use super::*;
use ai_center_server::{
    auth::AuthRuntime,
    company::{self, models::CompanyInput},
    config::AuthMode,
    routes,
};
use anyhow::Context;
use axum::{extract::Request, response::Response};
use std::path::Path;

const WEB_URL: &str = "http://127.0.0.1:5183";
#[derive(Clone)]
struct Remote {
    page: Uuid,
    issue: Uuid,
    workspace: i64,
    admin: PgPool,
    revision: Arc<AtomicUsize>,
    linear_calls: Arc<AtomicUsize>,
    notion_calls: Arc<AtomicUsize>,
}
async fn provider(State(s): State<Remote>, request: Request) -> Response {
    let path = request.uri().path().to_owned();
    if path == "/fixture/advance" && request.method() == axum::http::Method::POST {
        // Advance only our synthetic clock boundary; cooldown itself is tested
        // by the separate real-transaction service suite, not skipped in the app.
        sqlx::query("update app.tool_source_references set last_attempt_at=clock_timestamp()-interval '61 seconds' where workspace_id=$1")
            .bind(s.workspace).execute(&s.admin).await.expect("fixture clock advance");
        s.revision.store(2, Ordering::SeqCst);
        return StatusCode::NO_CONTENT.into_response();
    }
    let revision = s.revision.load(Ordering::SeqCst);
    if path == "/linear/graphql" && request.method() == axum::http::Method::POST {
        let bytes = axum::body::to_bytes(request.into_body(), 16_000)
            .await
            .unwrap();
        let input: Value = serde_json::from_slice(&bytes).unwrap();
        assert!(
            input["query"]
                .as_str()
                .unwrap()
                .starts_with("query ExistingIssue")
        );
        assert_eq!(input["variables"]["id"], "PROD-42");
        s.linear_calls.fetch_add(1, Ordering::SeqCst);
        return Json(json!({"data":{"issue":{"id":s.issue,"identifier":"PROD-42","url":"https://linear.app/fictif/issue/PROD-42/spec","title":"[FICTIF] Ticket existant","description":"[FICTIF] Les crédits restent disponibles sans expiration.","updatedAt":"2026-09-24T00:00:00Z","team":{"id":"20000000-0000-4000-8000-000000000001"},"state":{"id":"20000000-0000-4000-8000-000000000002","name":"[FICTIF] À faire","type":"unstarted"}}}})).into_response();
    }
    if request.method() != axum::http::Method::GET {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    if path == format!("/notion/v1/pages/{}", s.page) {
        s.notion_calls.fetch_add(1, Ordering::SeqCst);
        return Json(json!({"object":"page","id":s.page,"archived":false,"in_trash":false,"last_edited_time":format!("2026-09-24T00:0{revision}:00Z"),"parent":{"type":"workspace","workspace":true},"properties":{"Name":{"type":"title","title":[{"plain_text":"[FICTIF] Spécification existante"}]}}})).into_response();
    }
    if path == format!("/notion/v1/pages/{}/markdown", s.page) {
        assert_eq!(request.uri().query(), Some("include_transcript=false"));
        s.notion_calls.fetch_add(1, Ordering::SeqCst);
        return Json(json!({"object":"page_markdown","id":s.page,"markdown":format!("[FICTIF] Version distante {revision}. Les décisions restent traçables."),"truncated":revision==2,"unknown_block_ids":[]})).into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}

async fn browser() -> Result<()> {
    let f = fixture().await?;
    company::update(
        &f.owner,
        CompanyInput {
            name: "[FICTIF] Société sources".into(),
            description: "Recette des documents existants".into(),
        },
        true,
        None,
    )
    .await?;
    let project = service::create_project(
        &f.owner,
        CreateProject {
            name: "[FICTIF] Contexte outils".into(),
            objective: "Préparer le travail à partir de documents existants".into(),
        },
    )
    .await?;
    let mut connections = Vec::new();
    for provider in ["linear", "notion"] {
        let id = Uuid::new_v4();
        let mut input = connection(id);
        input.provider = provider.into();
        input.name = format!("[FICTIF] {provider}");
        input.allow_existing_reads = Some(true);
        work_tools::save_connection(&f.owner, input, None).await?;
        connections.push(id);
    }
    let mut tasks = Tasks::default();
    let remote = Remote {
        page: Uuid::new_v4(),
        issue: Uuid::new_v4(),
        workspace: f.owner.workspace_internal_id.context("fixture scope")?,
        admin: f.admin.clone(),
        revision: Arc::new(AtomicUsize::new(1)),
        linear_calls: Arc::new(AtomicUsize::new(0)),
        notion_calls: Arc::new(AtomicUsize::new(0)),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let provider_url = format!("http://{}/", listener.local_addr()?);
    let app = Router::new()
        .fallback(any(provider))
        .with_state(remote.clone());
    tasks.0.push(tokio::spawn(async move {
        axum::serve(listener, app).await.expect("fixture provider");
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let api = format!("http://{}", listener.local_addr()?);
    let auth = Arc::new(AuthRuntime::new(
        AuthMode::Local,
        f.owner.workspace_id,
        f.owner.actor_id,
        None,
    )?);
    let app = routes::router_with_loopback_source_reader(
        Arc::new(f.owner.clone()),
        auth,
        vec![WEB_URL.parse()?],
        &provider_url,
    )?
    .layer(axum::middleware::from_fn(block_official_reads));
    tasks.0.push(tokio::spawn(async move {
        axum::serve(listener, app).await.expect("fixture API");
    }));
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .context("repo root")?;
    let env = [
        ("VITE_API_URL", api.clone()),
        ("VITE_WORKSPACE_ID", f.owner.workspace_id.to_string()),
        ("VITE_SUPABASE_URL", String::new()),
        ("VITE_SUPABASE_ANON_KEY", String::new()),
        ("TAURI_DEV_HOST", String::new()),
        ("E2E_API_URL", api.clone()),
        ("E2E_WEB_URL", WEB_URL.into()),
        ("E2E_WORKSPACE_ID", f.owner.workspace_id.to_string()),
        ("E2E_SOURCE_PROJECT", project.public_id.to_string()),
        ("E2E_SOURCE_PROVIDER", provider_url),
        ("E2E_SOURCE_PAGE", remote.page.to_string()),
        ("E2E_LINEAR_CONNECTION_ID", connections[0].to_string()),
        ("E2E_NOTION_CONNECTION_ID", connections[1].to_string()),
    ];
    drop(
        std::net::TcpListener::bind("127.0.0.1:5183")
            .context("source-browser requires free Vite port")?,
    );
    let mut vite = node(
        root,
        &[
            "node_modules/vite/bin/vite.js",
            "--config",
            "apps/web/vite.integration.config.ts",
            "apps/web",
            "--host",
            "127.0.0.1",
            "--port",
            "5183",
            "--strictPort",
        ],
        &env,
        Some(".run/source-browser-web.log"),
    )?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .no_proxy()
        .build()?;
    tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            ensure!(vite.child.try_wait()?.is_none(), "Vite stopped");
            if client
                .get(WEB_URL)
                .send()
                .await
                .is_ok_and(|r| r.status().is_success())
            {
                return Ok::<(), anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .context("source browser web readiness")??;
    let mut playwright = node(
        root,
        &[
            "node_modules/@playwright/test/cli.js",
            "test",
            "--config=apps/web/playwright.source-real.config.ts",
        ],
        &env,
        None,
    )?;
    let status = tokio::time::timeout(Duration::from_secs(150), playwright.child.wait())
        .await
        .context("source browser deadline")??;
    ensure!(status.success(), "source browser scenario failed");
    assert_eq!(
        remote.linear_calls.load(Ordering::SeqCst),
        1,
        "lost response and GETs never repeat provider read"
    );
    assert_eq!(
        remote.notion_calls.load(Ordering::SeqCst),
        6,
        "two explicit three-request observations"
    );
    let counts:(i64,i64)=sqlx::query_as("select (select count(*) from app.tool_source_references where workspace_id=$1),(select count(*) from app.tool_source_observations where workspace_id=$1)").bind(remote.workspace).fetch_one(&f.admin).await?;
    assert_eq!(counts, (2, 3));
    drop(tasks);
    f.owner.pool.close().await;
    f.admin.close().await;
    Ok(())
}

#[tokio::test]
#[ignore = "requires guarded source-browser phase and local Chromium"]
async fn existing_sources_browser() -> Result<()> {
    tokio::time::timeout(Duration::from_secs(240), browser())
        .await
        .context("source browser harness deadline")?
}
