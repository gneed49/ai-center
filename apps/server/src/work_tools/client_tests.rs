use super::*;
use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::any,
};
use std::sync::{Arc, Mutex};
type Requests = Arc<Mutex<Vec<(String, Value)>>>;
#[derive(Clone)]
struct Fixture {
    provider: String,
    target: String,
    id: String,
    mode: &'static str,
    requests: Requests,
}
async fn respond(State(f): State<Fixture>, request: Request<Body>) -> axum::response::Response {
    let path = request.uri().path().to_owned();
    let method = request.method().clone();
    assert_eq!(
        request
            .headers()
            .get("authorization")
            .and_then(|h| h.to_str().ok()),
        Some(if f.provider == "linear" {
            "synthetic-secret"
        } else {
            "Bearer synthetic-secret"
        })
    );
    if f.provider == "notion" {
        assert_eq!(
            request
                .headers()
                .get("Notion-Version")
                .and_then(|h| h.to_str().ok()),
            Some("2026-03-11")
        );
    }
    let bytes = axum::body::to_bytes(request.into_body(), 100_000)
        .await
        .expect("request");
    let input: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    f.requests
        .lock()
        .expect("requests")
        .push((path.clone(), input.clone()));
    match f.mode {
        "redirect" => {
            return (
                StatusCode::TEMPORARY_REDIRECT,
                [("location", "https://unexpected.example/collect")],
            )
                .into_response();
        }
        "refused" => {
            return (
                StatusCode::UNAUTHORIZED,
                "synthetic-secret echoed by unsafe remote",
            )
                .into_response();
        }
        "server" => return StatusCode::BAD_GATEWAY.into_response(),
        "timeout" => tokio::time::sleep(Duration::from_millis(150)).await,
        "graphql_error" => {
            return axum::Json(json!({"errors":[{"message":"synthetic-secret"}],"data":null}))
                .into_response();
        }
        "oversized" => return "x".repeat(MAX_RESPONSE + 1).into_response(),
        "malformed" => return axum::Json(json!({"ok":true})).into_response(),
        _ => {}
    }
    let value = match f.provider.as_str() {
        "notion" => {
            if path.ends_with("/markdown") {
                json!({"object":"page_markdown","id":f.id,"markdown":"# [FICTIF] content","truncated":f.mode=="truncated","unknown_block_ids":[]})
            } else {
                json!({"object":"page","id":f.id,"parent":{"page_id":f.target},"properties":{"title":{"title":[{"plain_text":"[FICTIF] title"}]}},"archived":false,"last_edited_time":"2026-09-21T12:00:00Z"})
            }
        }
        "linear" => {
            let issue = json!({"id":f.id,"url":"https://linear.app/fictif/issue/TEST-1","title":"[FICTIF] title","description":"# [FICTIF] content","updatedAt":"2026-09-21T12:00:00Z","team":{"id":f.target}});
            if input["query"]
                .as_str()
                .is_some_and(|query| query.contains("issueCreate"))
            {
                json!({"data":{"issueCreate":{"success":true,"issue":issue}}})
            } else {
                json!({"data":{"issue":issue}})
            }
        }
        _ => {
            json!({"number":42,"html_url":format!("https://github.com/{}/issues/42",f.target),"title":"[FICTIF] title","body":"# [FICTIF] content","updated_at":"2026-09-21T12:00:00Z"})
        }
    };
    (
        if method == Method::POST && f.provider != "linear" {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        axum::Json(value),
    )
        .into_response()
}
async fn server(
    provider: &str,
    mode: &'static str,
) -> (ToolClient, Fixture, tokio::task::JoinHandle<()>) {
    let f = Fixture {
        provider: provider.into(),
        target: if provider == "github" {
            "fictif/repository".into()
        } else {
            Uuid::new_v4().to_string()
        },
        id: if provider == "github" {
            "42".into()
        } else {
            Uuid::new_v4().to_string()
        },
        mode,
        requests: Arc::default(),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let client = ToolClient::loopback(
        &format!("http://{}/", listener.local_addr().expect("address")),
        Duration::from_millis(80),
    )
    .expect("client");
    let router = Router::new().fallback(any(respond)).with_state(f.clone());
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve");
    });
    (client, f, task)
}
#[tokio::test]
async fn official_create_and_read_contracts_are_bounded_and_target_checked() {
    for provider in ["notion", "linear", "github"] {
        let (client, f, task) = server(provider, "success").await;
        let secret = SecretString::from("synthetic-secret");
        let receipt = client
            .create(
                provider,
                &secret,
                &f.target,
                "[FICTIF] title",
                "# [FICTIF] content",
            )
            .await
            .expect("create");
        assert_eq!(receipt.external_id, f.id);
        assert_eq!(receipt.complete, provider != "notion");
        let read = client
            .read(provider, &secret, &f.target, &receipt.external_id)
            .await
            .expect("read");
        assert!(read.complete);
        let requests = f.requests.lock().expect("requests");
        if provider == "notion" {
            assert!(requests[0].1.get("allow_async").is_none());
            assert_eq!(requests[0].1["markdown"], "# [FICTIF] content");
        }
        drop(requests);
        task.abort();
    }
}
#[tokio::test]
async fn remote_failures_are_sanitized_and_ambiguous_create_is_not_retried() {
    for (mode, ambiguous) in [
        ("refused", false),
        ("server", true),
        ("redirect", true),
        ("graphql_error", true),
        ("oversized", true),
        ("malformed", true),
        ("timeout", true),
    ] {
        let (client, f, task) = server("linear", mode).await;
        let error = client
            .create(
                "linear",
                &SecretString::from("synthetic-secret"),
                &f.target,
                "[FICTIF] title",
                "content",
            )
            .await
            .err()
            .expect("failure");
        assert_eq!(error.ambiguous, ambiguous, "{mode}");
        assert!(!format!("{error:?}").contains("synthetic-secret"));
        assert_eq!(
            f.requests.lock().expect("requests").len(),
            1,
            "No automatic remote retry"
        );
        task.abort();
    }
}
#[tokio::test]
async fn incomplete_notion_reads_and_foreign_targets_are_not_complete_evidence() {
    let (client, f, task) = server("notion", "truncated").await;
    let receipt = client
        .read(
            "notion",
            &SecretString::from("synthetic-secret"),
            &f.target,
            &f.id,
        )
        .await
        .expect("read");
    assert!(!receipt.complete);
    assert!(
        client
            .read(
                "notion",
                &SecretString::from("synthetic-secret"),
                &Uuid::new_v4().to_string(),
                &f.id
            )
            .await
            .is_err()
    );
    task.abort();
    assert!(ToolClient::loopback("https://api.notion.com/", Duration::from_secs(1)).is_err());
    assert!(ToolClient::loopback("http://127.0.0.1:1234/x", Duration::from_secs(1)).is_err());
}
