//! [FICTIF] HTTP contracts; no database, official host, real credential or IA.
use super::super::{
    locator::{LinearIssueKey, parse_locator},
    models::{MAX_TEXT_BYTES, OmissionReason, SourceCoverage, SourceMetadata, ToolProvider},
};
use super::*;
use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{Response, StatusCode},
    routing::any,
};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

const ISSUE: &str = "10000000-0000-4000-8000-000000000001";
const TEAM: &str = "20000000-0000-4000-8000-000000000001";
type Reply = (Response<Body>, Duration);
#[derive(Clone)]
struct Fake {
    replies: Arc<Mutex<VecDeque<Reply>>>,
    seen: Arc<Mutex<Vec<(String, String, Value)>>>,
}
async fn handler(State(state): State<Fake>, request: Request) -> Response<Body> {
    let method = request.method().to_string();
    let path = request.uri().to_string();
    let bytes = axum::body::to_bytes(request.into_body(), 262_144)
        .await
        .unwrap();
    let input = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    state.seen.lock().unwrap().push((method, path, input));
    let next = state.replies.lock().unwrap().pop_front();
    let Some((reply, delay)) = next else {
        return Response::builder().status(500).body(Body::empty()).unwrap();
    };
    tokio::time::sleep(delay).await;
    reply
}
fn response(body: Value) -> Response<Body> {
    use axum::response::IntoResponse;
    axum::Json(body).into_response()
}
async fn start(
    replies: Vec<(Response<Body>, Duration)>,
    request_timeout: Duration,
    deadline: Duration,
) -> (ExistingToolReader, Fake, tokio::task::JoinHandle<()>) {
    let state = Fake {
        replies: Arc::new(Mutex::new(replies.into())),
        seen: Arc::new(Mutex::new(vec![])),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let reader = ExistingToolReader::loopback(
        &format!("http://{}/", listener.local_addr().unwrap()),
        request_timeout,
        deadline,
    )
    .unwrap();
    let app = Router::new()
        .fallback(any(handler))
        .with_state(state.clone());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (reader, state, task)
}
fn issue() -> Value {
    json!({"data":{"issue":{"id":ISSUE,"identifier":"PROD-42","url":"https://linear.app/fictif/issue/PROD-42/title","title":"[FICTIF] Fidélité du contexte","description":"[FICTIF] Passage **observé**.","updatedAt":"2026-09-24T01:00:00Z","team":{"id":TEAM},"state":{"id":"30000000-0000-4000-8000-000000000001","name":"En cours","type":"started"}}}})
}
fn secret() -> SecretString {
    SecretString::from("synthetic-test-value-only")
}
fn linear() -> SourceLocator {
    parse_locator(
        ToolProvider::Linear,
        "https://linear.app/fictif/issue/PROD-42/title",
    )
    .unwrap()
}
fn available(
    outcome: Result<ExistingReadOutcome, Failure>,
) -> super::super::models::ExistingToolSnapshot {
    match outcome.unwrap() {
        ExistingReadOutcome::Available(snapshot) => *snapshot,
        ExistingReadOutcome::Unavailable { .. } => panic!("expected an available fixture"),
    }
}
fn failure_code(outcome: Result<ExistingReadOutcome, Failure>) -> Code {
    match outcome {
        Err(failure) => failure.code,
        Ok(_) => panic!("expected a refusal"),
    }
}
#[tokio::test]
async fn linear_is_one_parameterized_query_and_preserves_identity_state_and_null_description() {
    let mut value = issue();
    value["data"]["issue"]["description"] = Value::Null;
    let (reader, state, http) = start(
        vec![(response(value), Duration::ZERO)],
        Duration::from_secs(2),
        Duration::from_secs(3),
    )
    .await;
    let snapshot = available(reader.read_existing(&secret(), &linear()).await);
    assert_eq!(snapshot.external_id.to_string(), ISSUE);
    assert_eq!(snapshot.body_markdown, "");
    assert_eq!(snapshot.coverage, SourceCoverage::Complete);
    assert_eq!(
        snapshot.canonical_url,
        "https://linear.app/fictif/issue/PROD-42"
    );
    assert!(matches!(
        snapshot.metadata,
        SourceMetadata::Linear { state: Some(_), .. }
    ));
    let seen = state.seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].0, "POST");
    assert_eq!(seen[0].1, "/linear/graphql");
    assert_eq!(seen[0].2["variables"]["id"], "PROD-42");
    let query = seen[0].2["query"].as_str().unwrap();
    assert!(query.starts_with("query "));
    assert!(!query.contains("mutation"));
    assert!(!query.contains("PROD-42"));
    http.abort();
}
#[tokio::test]
async fn linear_refuses_incoherent_identity_and_graphql_partial_success() {
    let mut wrong_id = issue();
    wrong_id["data"]["issue"]["identifier"] = json!("PROD-43");
    let mut wrong_scope = issue();
    wrong_scope["data"]["issue"]["url"] = json!("https://linear.app/other/issue/PROD-42");
    let mut wrong_origin = issue();
    wrong_origin["data"]["issue"]["url"] =
        json!("https://linear.app.evil.example/fictif/issue/PROD-42");
    let mut mixed = issue();
    mixed["errors"] =
        json!([{"message":"unsafe payload must never escape","extensions":{"code":"FORBIDDEN"}}]);
    for (value, expected) in [
        (wrong_id, Code::RemoteIdentityMismatch),
        (wrong_scope, Code::RemoteIdentityMismatch),
        (wrong_origin, Code::RemoteIdentityMismatch),
        (mixed, Code::RemoteResponseInvalid),
    ] {
        let (reader, state, http) = start(
            vec![(response(value), Duration::ZERO)],
            Duration::from_secs(2),
            Duration::from_secs(3),
        )
        .await;
        assert_eq!(
            failure_code(reader.read_existing(&secret(), &linear()).await),
            expected
        );
        assert_eq!(state.seen.lock().unwrap().len(), 1);
        http.abort();
    }
    let (reader, _, http) = start(
        vec![(response(issue()), Duration::ZERO)],
        Duration::from_secs(2),
        Duration::from_secs(3),
    )
    .await;
    let wrong = SourceLocator::LinearIssue {
        key: LinearIssueKey::Id(Uuid::new_v4()),
        expected_workspace_slug: None,
    };
    assert_eq!(
        failure_code(reader.read_existing(&secret(), &wrong).await),
        Code::RemoteIdentityMismatch
    );
    http.abort();
}
#[tokio::test]
async fn text_budget_uses_utf8_bytes_and_unknown_date_is_not_invented() {
    let mut value = issue();
    value["data"]["issue"]["description"] = json!("漢".repeat(35_000));
    value["data"]["issue"]["updatedAt"] = Value::Null;
    let (reader, state, http) = start(
        vec![(response(value), Duration::ZERO)],
        Duration::from_secs(2),
        Duration::from_secs(3),
    )
    .await;
    let snapshot = available(reader.read_existing(&secret(), &linear()).await);
    assert!(snapshot.title.len() + snapshot.body_markdown.len() <= MAX_TEXT_BYTES);
    assert!(snapshot.body_markdown.ends_with('漢'));
    assert_eq!(snapshot.coverage, SourceCoverage::Partial);
    assert!(snapshot.remote_updated_at.is_none());
    assert!(
        snapshot
            .omission_reasons
            .contains(&OmissionReason::LocalTextLimit)
    );
    assert!(
        snapshot
            .omission_reasons
            .contains(&OmissionReason::RemoteDateUnavailable)
    );
    assert_eq!(state.seen.lock().unwrap().len(), 1);
    http.abort();
}
fn page(parent: Value) -> Value {
    let mut value = json!({"object":"page","id":ISSUE,"archived":false,"in_trash":false,"last_edited_time":"2026-09-24T01:00:00Z","properties":{"Un nom changé":{"type":"title","title":[{"plain_text":"[FICTIF] Source Notion"}]}}});
    value["parent"] = parent;
    value
}
fn markdown() -> Value {
    json!({"object":"page_markdown","id":ISSUE,"markdown":"[FICTIF] Texte de la page","truncated":false,"unknown_block_ids":[]})
}
fn notion() -> SourceLocator {
    SourceLocator::NotionPage {
        id: Uuid::parse_str(ISSUE).unwrap(),
    }
}
#[tokio::test]
async fn notion_accepts_supported_parents_and_reads_no_parent_or_transcript() {
    for parent in [
        json!({"type":"workspace","workspace":true}),
        json!({"type":"page_id","page_id":TEAM}),
        json!({"type":"data_source_id","data_source_id":TEAM}),
    ] {
        let p = page(parent);
        let (reader, state, http) = start(
            vec![
                (response(p.clone()), Duration::ZERO),
                (response(markdown()), Duration::ZERO),
                (response(p), Duration::ZERO),
            ],
            Duration::from_secs(2),
            Duration::from_secs(3),
        )
        .await;
        let snapshot = available(reader.read_existing(&secret(), &notion()).await);
        assert_eq!(snapshot.title, "[FICTIF] Source Notion");
        assert_eq!(snapshot.coverage, SourceCoverage::Complete);
        let seen = state.seen.lock().unwrap();
        assert_eq!(seen.len(), 3);
        assert!(seen.iter().all(|request| request.0 == "GET"));
        assert_eq!(
            seen[1].1,
            format!("/notion/v1/pages/{ISSUE}/markdown?include_transcript=false")
        );
        assert_eq!(seen[0].1, seen[2].1);
        http.abort();
    }
}
#[tokio::test]
async fn notion_reports_unknown_content_and_refuses_a_mid_read_change() {
    let p = page(json!({"type":"workspace","workspace":true}));
    let mut partial = markdown();
    partial["truncated"] = json!(true);
    partial["unknown_block_ids"] = json!([TEAM]);
    partial["markdown"] = json!("[FICTIF] <unknown url=\"https://evil.example/a\"/>");
    let (reader, state, http) = start(
        vec![
            (response(p.clone()), Duration::ZERO),
            (response(partial), Duration::ZERO),
            (response(p.clone()), Duration::ZERO),
        ],
        Duration::from_secs(2),
        Duration::from_secs(3),
    )
    .await;
    let snapshot = available(reader.read_existing(&secret(), &notion()).await);
    assert_eq!(snapshot.coverage, SourceCoverage::Partial);
    assert!(
        snapshot
            .omission_reasons
            .contains(&OmissionReason::UnknownBlocks)
    );
    assert!(
        snapshot
            .omission_reasons
            .contains(&OmissionReason::ProviderTruncated)
    );
    assert_eq!(state.seen.lock().unwrap().len(), 3);
    http.abort();
    let mut after = p.clone();
    after["last_edited_time"] = json!("2026-09-24T01:01:00Z");
    let (reader, _, http) = start(
        vec![
            (response(p), Duration::ZERO),
            (response(markdown()), Duration::ZERO),
            (response(after), Duration::ZERO),
        ],
        Duration::from_secs(2),
        Duration::from_secs(3),
    )
    .await;
    assert_eq!(
        failure_code(reader.read_existing(&secret(), &notion()).await),
        Code::RemoteChangedDuringRead
    );
    http.abort();
}
#[tokio::test]
async fn unavailable_is_distinct_from_transient_and_rate_limits_are_sanitized() {
    for (status, expected) in [
        (401, Code::RemoteAuthentication),
        (403, Code::RemotePermission),
        (404, Code::RemoteNotFound),
    ] {
        let reply = Response::builder()
            .status(status)
            .body(Body::from("NEVER RETURN THIS ERROR BODY"))
            .unwrap();
        let (reader, state, http) = start(
            vec![(reply, Duration::ZERO)],
            Duration::from_secs(2),
            Duration::from_secs(3),
        )
        .await;
        assert!(
            matches!(reader.read_existing(&secret(),&linear()).await,Ok(ExistingReadOutcome::Unavailable{code}) if code==expected)
        );
        assert_eq!(state.seen.lock().unwrap().len(), 1);
        http.abort();
    }
    for reply in [
        Response::builder()
            .status(429)
            .header("Retry-After", "999999999")
            .body(Body::empty())
            .unwrap(),
        response(
            json!({"errors":[{"message":"SECRET PROVIDER ERROR","extensions":{"code":"RATELIMITED"}}]}),
        ),
    ] {
        let (reader, state, http) = start(
            vec![(reply, Duration::ZERO)],
            Duration::from_secs(2),
            Duration::from_secs(3),
        )
        .await;
        let Err(failure) = reader.read_existing(&secret(), &linear()).await else {
            panic!("expected rate limit");
        };
        assert_eq!(failure.code, Code::RemoteRateLimit);
        assert!(failure.retryable);
        assert!((1..=86_400).contains(&failure.retry_after_seconds.unwrap()));
        assert!(!format!("{failure:?}").contains("SECRET"));
        assert_eq!(state.seen.lock().unwrap().len(), 1);
        http.abort();
    }
}
#[tokio::test]
async fn response_bounds_redirects_and_cumulative_deadline_stop_without_retry() {
    for (reply, expected) in [
        (
            Response::builder()
                .status(StatusCode::FOUND)
                .header("Location", "https://evil.example/private")
                .body(Body::empty())
                .unwrap(),
            Code::RemoteRedirectBlocked,
        ),
        (
            Response::builder()
                .body(Body::from("x".repeat(262_145)))
                .unwrap(),
            Code::RemoteResponseTooLarge,
        ),
        (
            Response::builder()
                .body(Body::from("{truncated-json"))
                .unwrap(),
            Code::RemoteResponseInvalid,
        ),
    ] {
        let (reader, state, http) = start(
            vec![(reply, Duration::ZERO)],
            Duration::from_secs(2),
            Duration::from_secs(3),
        )
        .await;
        assert_eq!(
            failure_code(reader.read_existing(&secret(), &linear()).await),
            expected
        );
        assert_eq!(state.seen.lock().unwrap().len(), 1);
        http.abort();
    }
    let p = page(json!({"type":"workspace","workspace":true}));
    let (reader, state, http) = start(
        vec![
            (response(p.clone()), Duration::from_millis(30)),
            (response(markdown()), Duration::from_millis(200)),
            (response(p), Duration::ZERO),
        ],
        Duration::from_secs(1),
        Duration::from_millis(80),
    )
    .await;
    assert_eq!(
        failure_code(reader.read_existing(&secret(), &notion()).await),
        Code::RemoteTimeout
    );
    assert_eq!(state.seen.lock().unwrap().len(), 2);
    http.abort();
}
#[test]
fn a_test_reader_never_accepts_an_external_host() {
    for base in [
        "https://api.linear.app/",
        "http://localhost:1234/",
        "http://127.0.0.1/",
        "http://user@127.0.0.1:1234/",
        "http://127.0.0.1:1234/base",
    ] {
        assert!(
            ExistingToolReader::loopback(base, Duration::from_secs(1), Duration::from_secs(1))
                .is_err()
        );
    }
}

#[tokio::test]
async fn notion_missing_date_and_empty_title_remain_observed_not_invented() {
    let mut p = page(json!({"type":"workspace","workspace":true}));
    p["last_edited_time"] = Value::Null;
    p["properties"]["Un nom changé"]["title"] = json!([]);
    let (reader, _, http) = start(
        vec![
            (response(p.clone()), Duration::ZERO),
            (response(markdown()), Duration::ZERO),
            (response(p), Duration::ZERO),
        ],
        Duration::from_secs(2),
        Duration::from_secs(3),
    )
    .await;
    let snapshot = available(reader.read_existing(&secret(), &notion()).await);
    assert_eq!(snapshot.title, "");
    assert!(snapshot.remote_updated_at.is_none());
    assert_eq!(snapshot.coverage, SourceCoverage::Partial);
    assert!(
        snapshot
            .omission_reasons
            .contains(&OmissionReason::RemoteDateUnavailable)
    );
    http.abort();
}

#[tokio::test]
async fn a_chunked_response_cannot_bypass_the_network_byte_limit() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let reader = ExistingToolReader::loopback(
        &format!("http://{}/", listener.local_addr().unwrap()),
        Duration::from_secs(2),
        Duration::from_secs(3),
    )
    .unwrap();
    let http = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buffer = [0; 4096];
        let _ = stream.read(&mut buffer).await;
        if stream.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Type: application/json\r\n\r\n").await.is_err() { return; }
        let chunk = format!("2000\r\n{}\r\n", "x".repeat(8192));
        for _ in 0..33 {
            if stream.write_all(chunk.as_bytes()).await.is_err() {
                return;
            }
        }
        let _ = stream.write_all(b"0\r\n\r\n").await;
    });
    assert_eq!(
        failure_code(reader.read_existing(&secret(), &linear()).await),
        Code::RemoteResponseTooLarge
    );
    http.abort();
}
