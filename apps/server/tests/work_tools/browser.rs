//! TP-011: actual web/API/PostgreSQL, deterministic AI and loopback-only tools.
//! Run only through the guarded ticket-browser phase, on an empty publication queue.
use super::*;
use ai_center_server::{auth::AuthRuntime, config::AuthMode, routes};
use anyhow::Context;
use axum::{extract::Request, middleware::Next, response::Response};
use std::{collections::HashSet, path::Path, process::Stdio};

const WEB_URL: &str = "http://127.0.0.1:5183";
const GITHUB_TARGET: &str = "fictif/repository";

#[derive(Clone)]
struct CapturedIssue {
    provider: &'static str,
    external_id: String,
    title: String,
    body: String,
}

#[derive(Clone)]
struct BrowserRemote {
    linear_team: Uuid,
    issues: Arc<Mutex<Vec<CapturedIssue>>>,
}

async fn provider(State(remote): State<BrowserRemote>, request: Request) -> Response {
    let path = request.uri().path().to_owned();
    if request.method() != axum::http::Method::POST {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    let Ok(bytes) = axum::body::to_bytes(request.into_body(), 100_000).await else {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    };
    let Ok(body) = serde_json::from_slice::<Value>(&bytes) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let (provider, input) = match path.as_str() {
        "/github/repos/fictif/repository/issues" => ("github", &body),
        "/linear/graphql"
            if body["query"].as_str().is_some_and(|query| {
                query.starts_with("mutation Publish(") && query.contains("issueCreate")
            }) && body["variables"]["input"]["teamId"] == remote.linear_team.to_string() =>
        {
            ("linear", &body["variables"]["input"])
        }
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    let field = if provider == "github" {
        "body"
    } else {
        "description"
    };
    let (Some(title), Some(content)) = (input["title"].as_str(), input[field].as_str()) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let mut issues = remote.issues.lock().expect("fictitious issues");
    let number = issues
        .iter()
        .filter(|issue| issue.provider == provider)
        .count()
        + 1;
    let external_id = if provider == "linear" {
        Uuid::new_v4().to_string()
    } else {
        number.to_string()
    };
    issues.push(CapturedIssue {
        provider,
        external_id: external_id.clone(),
        title: title.into(),
        body: content.into(),
    });
    let value = if provider == "linear" {
        json!({"data":{"issueCreate":{"success":true,"issue":{
            "id":external_id,"identifier":format!("FICTIF-{number}"),
            "url":format!("https://linear.app/fictitious/issue/FICTIF-{number}/ticket"),
            "title":title,"description":content,"team":{"id":remote.linear_team},
            "updatedAt":"2026-09-24T00:00:00Z"}}}})
    } else {
        json!({"number":number,"html_url":format!("https://github.com/{GITHUB_TARGET}/issues/{number}"),
            "title":title,"body":content,"updated_at":"2026-09-24T00:00:00Z"})
    };
    Json(value).into_response()
}

// The browser scenario never invokes a production API adapter from these routes.
// Publication creation is only drained by our explicitly loopback client below.
async fn block_official_reads(request: Request, next: Next) -> Response {
    let path = request.uri().path();
    if request.method() == axum::http::Method::POST
        && ((path.starts_with("/api/work-tools/connections/") && path.ends_with("/test"))
            || (path.starts_with("/api/publications/")
                && (path.ends_with("/refresh") || path.ends_with("/reconcile")))
            || path.ends_with("/code-observations"))
    {
        return StatusCode::FORBIDDEN.into_response();
    }
    next.run(request).await
}

#[derive(Default)]
struct Tasks(Vec<tokio::task::JoinHandle<()>>);
impl Drop for Tasks {
    fn drop(&mut self) {
        for task in &self.0 {
            task.abort();
        }
    }
}

struct Process {
    child: tokio::process::Child,
    #[cfg(unix)]
    group: nix::unistd::Pid,
}
impl Drop for Process {
    fn drop(&mut self) {
        #[cfg(unix)]
        let _ = nix::sys::signal::killpg(self.group, nix::sys::signal::Signal::SIGKILL);
        let _ = self.child.start_kill();
    }
}

fn node(
    root: &Path,
    args: &[&str],
    environment: &[(&str, String)],
    log: Option<&str>,
) -> Result<Process> {
    let mut command = std::process::Command::new("node");
    command
        .current_dir(root)
        .args(args)
        .env_clear()
        .stdin(Stdio::null());
    for name in [
        "PATH",
        "HOME",
        "XDG_CACHE_HOME",
        "PLAYWRIGHT_BROWSERS_PATH",
        "TMPDIR",
        "CI",
    ] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    for (name, value) in environment {
        command.env(name, value);
    }
    if let Some(log) = log {
        let output = std::fs::File::create(root.join(log))?;
        command
            .stdout(Stdio::from(output.try_clone()?))
            .stderr(Stdio::from(output));
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut command = tokio::process::Command::from(command);
    command.kill_on_drop(true);
    let child = command.spawn().context("start local browser tool")?;
    #[cfg(unix)]
    let group = nix::unistd::Pid::from_raw(i32::try_from(child.id().context("child process id")?)?);
    Ok(Process {
        child,
        #[cfg(unix)]
        group,
    })
}

async fn configure(owner: &AppState, kind: &str, provider: &str, target: &str) -> Result<Uuid> {
    let id = Uuid::new_v4();
    let mut input = connection(id);
    input.provider = provider.into();
    input.name = format!("[FICTIF] {provider} navigateur");
    work_tools::save_connection(owner, input, None).await?;
    artifacts::set_destination(
        owner,
        None,
        SetDestination {
            artifact_type: kind.into(),
            provider: provider.into(),
            target_id: Some(target.into()),
            label: format!("[FICTIF] {provider}"),
            expected_revision: 0,
        },
        None,
    )
    .await?;
    Ok(id)
}

async fn browser() -> Result<()> {
    let f = fixture().await?;
    let pending: i64 = sqlx::query_scalar(
        "select count(*) from app.publication_jobs where status in ('queued','processing')",
    )
    .fetch_one(&f.admin)
    .await?;
    ensure!(
        pending == 0,
        "ticket-browser requires a fresh isolated queue; it must precede integration fixtures"
    );
    let team = Uuid::new_v4();
    let linear = configure(&f.owner, "product_tickets", "linear", &team.to_string()).await?;
    let github = configure(&f.owner, "technical_tickets", "github", GITHUB_TARGET).await?;
    let mut tasks = Tasks::default();
    let remote = BrowserRemote {
        linear_team: team,
        issues: Arc::new(Mutex::new(Vec::new())),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let client = ToolClient::loopback(
        &format!("http://{}/", listener.local_addr()?),
        Duration::from_secs(5),
    )?;
    let app = Router::new()
        .fallback(any(provider))
        .with_state(remote.clone());
    tasks.0.push(tokio::spawn(async move {
        axum::serve(listener, app).await.expect("loopback provider");
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let api = format!("http://{}", listener.local_addr()?);
    let auth = Arc::new(AuthRuntime::new(
        AuthMode::Local,
        f.owner.workspace_id,
        f.owner.actor_id,
        None,
    )?);
    let app = routes::router(
        Arc::new(f.owner.clone()),
        auth,
        None,
        vec![WEB_URL.parse()?],
    )
    .layer(axum::middleware::from_fn(block_official_reads));
    tasks.0.push(tokio::spawn(async move {
        axum::serve(listener, app).await.expect("local API");
    }));
    let worker_error = Arc::new(Mutex::new(None));
    let failures = worker_error.clone();
    let state = f.owner.clone();
    tasks.0.push(tokio::spawn(async move {
        loop {
            if let Err(error) = worker::drain_one(&state, &client).await {
                *failures.lock().expect("worker error") = Some(error.to_string());
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }));
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .context("repository root")?;
    std::fs::create_dir_all(root.join(".run"))?;
    let env = [
        ("VITE_API_URL", api.clone()),
        ("VITE_WORKSPACE_ID", f.owner.workspace_id.to_string()),
        ("VITE_SUPABASE_URL", String::new()),
        ("VITE_SUPABASE_ANON_KEY", String::new()),
        ("TAURI_DEV_HOST", String::new()),
        ("E2E_API_URL", api.clone()),
        ("E2E_WEB_URL", WEB_URL.into()),
        ("E2E_WORKSPACE_ID", f.owner.workspace_id.to_string()),
        ("E2E_CONNECTION_ID", github.to_string()),
        ("E2E_TARGET_ID", GITHUB_TARGET.into()),
        ("E2E_GITHUB_CONNECTION_ID", github.to_string()),
        ("E2E_GITHUB_TARGET_ID", GITHUB_TARGET.into()),
        ("E2E_LINEAR_CONNECTION_ID", linear.to_string()),
        ("E2E_LINEAR_TARGET_ID", team.to_string()),
    ];
    // Never run the scenario against an unrelated server already using this port.
    drop(
        std::net::TcpListener::bind("127.0.0.1:5183")
            .context("ticket-browser requires free Vite port 5183")?,
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
        Some(".run/ticket-browser-web.log"),
    )?;
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .no_proxy()
        .build()?;
    tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            ensure!(
                vite.child.try_wait()?.is_none(),
                "Vite exited; inspect .run/ticket-browser-web.log"
            );
            if http
                .get(WEB_URL)
                .send()
                .await
                .is_ok_and(|response| response.status().is_success())
                && http
                    .get(format!("{api}/api/health"))
                    .send()
                    .await
                    .is_ok_and(|response| response.status().is_success())
            {
                return Ok::<_, anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .context("local web/API readiness timed out")??;
    let mut playwright = node(
        root,
        &[
            "node_modules/@playwright/test/cli.js",
            "test",
            "--config=apps/web/playwright.ticket-real.config.ts",
        ],
        &env,
        None,
    )?;
    let status = tokio::time::timeout(Duration::from_secs(150), playwright.child.wait())
        .await
        .context("ticket browser timed out")??;
    ensure!(status.success(), "ticket Playwright scenario failed");
    drop(tasks);
    let failure = worker_error.lock().expect("worker error").clone();
    ensure!(failure.is_none(), "loopback worker failed: {failure:?}");
    assert_receipts(&f, &remote).await?;
    f.owner.pool.close().await;
    f.admin.close().await;
    Ok(())
}

async fn assert_receipts(f: &Fixture, remote: &BrowserRemote) -> Result<()> {
    let issues = remote.issues.lock().expect("fictitious issues").clone();
    assert_eq!(
        issues.len(),
        5,
        "response loss/reload must not duplicate creates"
    );
    assert_eq!(
        issues
            .iter()
            .filter(|issue| issue.provider == "linear")
            .count(),
        2
    );
    assert_eq!(
        issues
            .iter()
            .filter(|issue| issue.provider == "github")
            .count(),
        3
    );
    assert_eq!(
        issues
            .iter()
            .map(|issue| (issue.provider, &issue.external_id))
            .collect::<HashSet<_>>()
            .len(),
        5
    );
    let rows: Vec<(Uuid, String, i16, String, String, String)> = sqlx::query_as("select public_id,provider,source_ticket_index,external_id,title,body_markdown from app.publication_jobs where workspace_id=$1 and status='succeeded' order by provider,source_ticket_index")
        .bind(f.owner.workspace_internal_id).fetch_all(&f.admin).await?;
    assert_eq!(rows.len(), 5);
    for (id, provider, index, external_id, title, body) in rows {
        assert!((0..if provider == "linear" { 2 } else { 3 }).contains(&index));
        assert!(body.contains(&format!("AI Center publication: {id}")));
        let received = issues
            .iter()
            .find(|issue| issue.provider == provider && issue.external_id == external_id)
            .context("corresponding loopback issue")?;
        assert_eq!(received.title, title);
        assert_eq!(received.body, body);
    }
    let counts: (i64, i64, i64) = sqlx::query_as("select count(*),count(distinct project_id),(select count(*) from app.publication_observations where workspace_id=$1) from app.publication_jobs where workspace_id=$1")
        .bind(f.owner.workspace_internal_id).fetch_one(&f.admin).await?;
    assert_eq!(
        counts,
        (5, 1, 5),
        "five exact receipts in the same PM/lead project"
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires guarded ticket-browser phase and installed local Chromium"]
async fn ticket_publication_browser() -> Result<()> {
    tokio::time::timeout(Duration::from_secs(240), browser())
        .await
        .context("ticket browser harness exceeded four minutes")?
}
