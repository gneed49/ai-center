use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::Duration,
};

use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{Response, header::HeaderValue},
};
use secrecy::SecretString;
use serde_json::{Value, json};
use tokio::{net::TcpListener, sync::Mutex, task::JoinHandle, time::sleep};

use super::*;

const CONTRACT_TOKEN: &str = "github-contract-test-token";
const BASE_SHA: &str = "1111111111111111111111111111111111111111";
const HEAD_SHA: &str = "2222222222222222222222222222222222222222";
const COMMIT_SHA: &str = "3333333333333333333333333333333333333333";
const NEW_HEAD_SHA: &str = "4444444444444444444444444444444444444444";
const NEW_COMMIT_SHA: &str = "5555555555555555555555555555555555555555";
const REPOSITORY_PATH: &str = "/repos/acme/context";
const PULL_PATH: &str = "/repos/acme/context/pulls/7";
const COMMITS_PATH: &str = "/repos/acme/context/pulls/7/commits?per_page=100";
const FILES_PATH: &str = "/repos/acme/context/pulls/7/files?per_page=100";
const COMMIT_PATH: &str = "/repos/acme/context/commits/2222222222222222222222222222222222222222";
const CHECKS_PATH: &str =
    "/repos/acme/context/commits/2222222222222222222222222222222222222222/check-runs?per_page=100";
const NEW_CHECKS_PATH: &str =
    "/repos/acme/context/commits/4444444444444444444444444444444444444444/check-runs?per_page=100";

#[derive(Clone)]
struct MockState {
    responses: Arc<Mutex<HashMap<String, VecDeque<MockResponse>>>>,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
}

struct MockResponse {
    status: StatusCode,
    headers: Vec<(String, String)>,
    body: String,
    delay: Duration,
}

impl MockResponse {
    fn json(status: StatusCode, body: &Value) -> Self {
        Self {
            status,
            headers: vec![("content-type".into(), "application/json".into())],
            body: body.to_string(),
            delay: Duration::ZERO,
        }
    }

    fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }

    fn delayed(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

#[derive(Debug)]
struct RecordedRequest {
    method: String,
    path_and_query: String,
    authorized: bool,
    accepts_github_json: bool,
    uses_expected_api_version: bool,
    if_none_match: Option<String>,
}

struct FakeGitHubServer {
    endpoint: String,
    state: MockState,
    task: JoinHandle<()>,
}

impl FakeGitHubServer {
    async fn start(routes: Vec<(&str, Vec<MockResponse>)>) -> Self {
        let state = MockState {
            responses: Arc::new(Mutex::new(
                routes
                    .into_iter()
                    .map(|(path, responses)| (path.to_owned(), responses.into()))
                    .collect(),
            )),
            requests: Arc::new(Mutex::new(Vec::new())),
        };
        let app = Router::new()
            .fallback(mock_github_handler)
            .with_state(state.clone());
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("fake GitHub listener should bind");
        let address = listener
            .local_addr()
            .expect("fake GitHub listener should expose its address");
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("fake GitHub server should run");
        });
        Self {
            endpoint: format!("http://{address}"),
            state,
            task,
        }
    }

    fn client(&self) -> GitHubClient {
        self.client_with_timeout(Duration::from_secs(2))
    }

    fn client_with_timeout(&self, timeout: Duration) -> GitHubClient {
        GitHubClient::for_contract_test(&self.endpoint, timeout)
            .expect("loopback contract client should build")
    }

    async fn requests(&self) -> Vec<RecordedRequest> {
        let mut requests = self.state.requests.lock().await;
        std::mem::take(&mut *requests)
    }
}

impl Drop for FakeGitHubServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn mock_github_handler(State(state): State<MockState>, request: Request) -> Response<Body> {
    let path_and_query = request
        .uri()
        .path_and_query()
        .map_or_else(|| request.uri().path().to_owned(), ToString::to_string);
    let headers = request.headers();
    let recorded = RecordedRequest {
        method: request.method().to_string(),
        path_and_query: path_and_query.clone(),
        authorized: headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            == Some(&format!("Bearer {CONTRACT_TOKEN}")),
        accepts_github_json: headers
            .get(header::ACCEPT)
            .and_then(|value| value.to_str().ok())
            == Some("application/vnd.github+json"),
        uses_expected_api_version: headers
            .get("x-github-api-version")
            .and_then(|value| value.to_str().ok())
            == Some(GITHUB_API_VERSION),
        if_none_match: headers
            .get(header::IF_NONE_MATCH)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
    };
    state.requests.lock().await.push(recorded);
    let response = state
        .responses
        .lock()
        .await
        .get_mut(&path_and_query)
        .and_then(VecDeque::pop_front);
    let Some(response) = response else {
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("unexpected fake GitHub request"))
            .expect("fallback response should build");
    };
    if !response.delay.is_zero() {
        sleep(response.delay).await;
    }
    let mut builder = Response::builder().status(response.status);
    for (name, value) in response.headers {
        builder = builder.header(name, value);
    }
    builder
        .body(Body::from(response.body))
        .expect("mock response should build")
}

fn contract_token() -> SecretString {
    SecretString::from(CONTRACT_TOKEN)
}

fn request_count(requests: &[RecordedRequest], path: &str) -> usize {
    requests
        .iter()
        .filter(|request| request.path_and_query == path)
        .count()
}

fn assert_no_forbidden_projection_keys(value: &Value) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                assert!(
                    !matches!(
                        key.as_str(),
                        "body" | "content" | "diff" | "diff_url" | "patch" | "patch_url"
                    ),
                    "forbidden GitHub field leaked into projection: {key}"
                );
                assert_no_forbidden_projection_keys(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                assert_no_forbidden_projection_keys(value);
            }
        }
        _ => {}
    }
}

#[test]
fn parses_only_canonical_github_pull_request_urls() {
    let identity =
        GitHubPullRequestIdentity::parse("https://github.com/openai/openai-rust/pull/42")
            .expect("canonical URL should parse");
    assert_eq!(identity.owner, "openai");
    assert_eq!(identity.repository, "openai-rust");
    assert_eq!(identity.number, 42);
    assert_eq!(
        identity.canonical_url(),
        "https://github.com/openai/openai-rust/pull/42"
    );
}

#[test]
fn rejects_non_canonical_urls_and_ssrf_shapes() {
    for value in [
        "http://github.com/acme/repo/pull/1",
        "https://example.com/acme/repo/pull/1",
        "https://github.com.evil.example/acme/repo/pull/1",
        "https://user@github.com/acme/repo/pull/1",
        "https://github.com:443/acme/repo/pull/1",
        "https://github.com/acme/repo/issues/1",
        "https://github.com/acme/repo/pull/1?diff=1",
        "https://github.com/acme/repo/pull/1#fragment",
        "https://github.com/acme/../pull/1",
        "https://github.com/acme/repo/pull/%2fetc",
    ] {
        assert!(GitHubPullRequestIdentity::parse(value).is_err(), "{value}");
    }
}

#[test]
fn production_origin_is_fixed_and_test_override_requires_literal_loopback() {
    let production = GitHubApiEndpoint::production().expect("production endpoint should be valid");
    assert_eq!(
        production
            .request_url("/repos/acme/context/pulls/7")
            .expect("production path should join")
            .as_str(),
        "https://api.github.com/repos/acme/context/pulls/7"
    );
    assert!(production.request_url("//evil.example/path").is_err());

    for valid in ["http://127.0.0.1:3210", "http://[::1]:3210"] {
        assert!(
            GitHubApiEndpoint::loopback_for_test(valid).is_ok(),
            "{valid}"
        );
    }
    for invalid in [
        "https://api.github.com",
        "http://localhost:3210",
        "http://127.0.0.1.evil.example:3210",
        "http://user@127.0.0.1:3210",
        "http://127.0.0.1:3210/base",
        "http://127.0.0.1:3210?query=1",
        "file:///tmp/github",
    ] {
        assert!(
            GitHubApiEndpoint::loopback_for_test(invalid).is_err(),
            "{invalid}"
        );
    }
}

fn happy_projection_routes() -> Vec<(&'static str, Vec<MockResponse>)> {
    vec![
        (
            REPOSITORY_PATH,
            vec![MockResponse::json(
                StatusCode::OK,
                &json!({
                    "id": 77,
                    "full_name": "acme/context",
                    "private": true
                }),
            )],
        ),
        (
            PULL_PATH,
            vec![
                MockResponse::json(
                    StatusCode::OK,
                    &json!({
                        "title": "Compile bounded context",
                        "state": "open",
                        "draft": false,
                        "merged": false,
                        "base": {"sha": BASE_SHA},
                        "head": {"sha": HEAD_SHA},
                        "created_at": "2026-08-24T10:00:00Z",
                        "updated_at": "2026-08-25T10:00:00Z",
                        "body": "provider body must not be imported",
                        "diff_url": "https://github.com/acme/context/pull/7.diff",
                        "patch_url": "https://github.com/acme/context/pull/7.patch"
                    }),
                )
                .with_header("etag", "\"pr-v1\""),
            ],
        ),
        (
            COMMITS_PATH,
            vec![MockResponse::json(
                StatusCode::OK,
                &json!([{
                    "sha": COMMIT_SHA,
                    "commit": {"message": "commit message must not be imported"}
                }]),
            )],
        ),
        (
            FILES_PATH,
            vec![MockResponse::json(
                StatusCode::OK,
                &json!([{
                    "filename": "apps/server/src/integrations/github.rs",
                    "patch": "patch must not be imported",
                    "contents_url": "https://api.github.com/repos/acme/context/contents/private"
                }]),
            )],
        ),
        (
            COMMIT_PATH,
            vec![MockResponse::json(
                StatusCode::OK,
                &json!({
                    "sha": HEAD_SHA,
                    "commit": {"author": {"date": "2026-08-25T10:00:00Z"}}
                }),
            )],
        ),
        (
            CHECKS_PATH,
            vec![MockResponse::json(
                StatusCode::OK,
                &json!({
                    "check_runs": [
                        {
                            "name": "contract",
                            "status": "completed",
                            "conclusion": "success",
                            "details_url": "https://github.com/acme/context/actions/runs/9",
                            "output": {"summary": "check output must not be imported"}
                        },
                        {
                            "name": "external",
                            "status": "completed",
                            "conclusion": null,
                            "details_url": "https://evil.example/run/1"
                        }
                    ]
                }),
            )],
        ),
    ]
}

fn pull_request_identity() -> GitHubPullRequestIdentity {
    GitHubPullRequestIdentity {
        owner: "acme".into(),
        repository: "context".into(),
        number: 7,
    }
}

fn previous_observation() -> GitHubPullRequestObservation {
    GitHubPullRequestObservation {
        identity: pull_request_identity(),
        canonical_url: "https://github.com/acme/context/pull/7".into(),
        title: "Compile bounded context".into(),
        state: "open".into(),
        draft: false,
        merged: false,
        base_sha: BASE_SHA.into(),
        head_sha: HEAD_SHA.into(),
        commits: vec![COMMIT_SHA.into()],
        changed_paths: vec!["apps/server/src/integrations/github.rs".into()],
        checks: vec![
            GitHubCheckObservation {
                name: "contract".into(),
                status: "completed".into(),
                conclusion: Some("success".into()),
                details_url: Some("https://github.com/acme/context/actions/runs/9".into()),
            },
            GitHubCheckObservation {
                name: "external".into(),
                status: "completed".into(),
                conclusion: None,
                details_url: None,
            },
        ],
        created_at: Some(
            "2026-08-24T10:00:00Z"
                .parse()
                .expect("created timestamp should parse"),
        ),
        updated_at: Some(
            "2026-08-25T10:00:00Z"
                .parse()
                .expect("updated timestamp should parse"),
        ),
        observed_at: "2026-08-25T10:01:00Z"
            .parse()
            .expect("observed timestamp should parse"),
    }
}

fn conditional_projection_routes(
    contract_conclusion: &str,
) -> Vec<(&'static str, Vec<MockResponse>)> {
    vec![
        (
            PULL_PATH,
            vec![MockResponse::json(StatusCode::NOT_MODIFIED, &json!(null))],
        ),
        (
            COMMITS_PATH,
            vec![MockResponse::json(
                StatusCode::OK,
                &json!([{ "sha": COMMIT_SHA }]),
            )],
        ),
        (
            FILES_PATH,
            vec![MockResponse::json(
                StatusCode::OK,
                &json!([{ "filename": "apps/server/src/integrations/github.rs" }]),
            )],
        ),
        (
            CHECKS_PATH,
            vec![MockResponse::json(
                StatusCode::OK,
                &json!({
                    "check_runs": [
                        {
                            "name": "contract",
                            "status": "completed",
                            "conclusion": contract_conclusion,
                            "details_url": "https://github.com/acme/context/actions/runs/9"
                        },
                        {
                            "name": "external",
                            "status": "completed",
                            "conclusion": null,
                            "details_url": "https://evil.example/run/1"
                        }
                    ]
                }),
            )],
        ),
    ]
}

fn assert_conditional_projection_requests(requests: &[RecordedRequest]) {
    assert_eq!(requests.len(), 4);
    for path in [PULL_PATH, COMMITS_PATH, FILES_PATH, CHECKS_PATH] {
        assert_eq!(request_count(requests, path), 1, "{path}");
    }
    let pull = requests
        .iter()
        .find(|request| request.path_and_query == PULL_PATH)
        .expect("pull request should be observed");
    assert_eq!(pull.if_none_match.as_deref(), Some("\"pr-v1\""));
    assert!(
        requests
            .iter()
            .filter(|request| request.path_and_query != PULL_PATH)
            .all(|request| request.if_none_match.is_none())
    );
}

#[tokio::test]
async fn observes_allowlisted_repository_pr_commit_files_and_checks_projection() {
    let server = FakeGitHubServer::start(happy_projection_routes()).await;

    let client = server.client();
    let repository = value_only(
        client
            .get_value(REPOSITORY_PATH, &contract_token(), None)
            .await
            .expect("repository metadata should be readable"),
    )
    .expect("repository response should contain JSON");
    assert_eq!(
        repository.get("full_name").and_then(Value::as_str),
        Some("acme/context")
    );
    let commit = value_only(
        client
            .get_value(COMMIT_PATH, &contract_token(), None)
            .await
            .expect("head commit metadata should be readable"),
    )
    .expect("commit response should contain JSON");
    assert_eq!(commit.get("sha").and_then(Value::as_str), Some(HEAD_SHA));

    let result = client
        .observe_pull_request(
            GitHubPullRequestIdentity {
                owner: "acme".into(),
                repository: "context".into(),
                number: 7,
            },
            None,
        )
        .await
        .expect("allowlisted projection should be observed");
    let GitHubObserveResult::Observed { observation, etag } = result else {
        panic!("fresh projection must not be not-modified");
    };
    assert_eq!(etag.as_deref(), Some("\"pr-v1\""));
    assert_eq!(observation.base_sha, BASE_SHA);
    assert_eq!(observation.head_sha, HEAD_SHA);
    assert_eq!(observation.commits, vec![COMMIT_SHA]);
    assert_eq!(
        observation.changed_paths,
        vec!["apps/server/src/integrations/github.rs"]
    );
    assert_eq!(observation.checks.len(), 2);
    assert_eq!(
        observation.checks[0].details_url.as_deref(),
        Some("https://github.com/acme/context/actions/runs/9")
    );
    assert!(observation.checks[1].details_url.is_none());

    let projection = serde_json::to_value(&observation).expect("projection should serialize");
    assert_no_forbidden_projection_keys(&projection);
    let serialized = projection.to_string();
    for forbidden in [
        "provider body must not be imported",
        "commit message must not be imported",
        "patch must not be imported",
        "check output must not be imported",
    ] {
        assert!(!serialized.contains(forbidden), "{forbidden}");
    }

    let requests = server.requests().await;
    assert_eq!(requests.len(), 6);
    for request in &requests {
        assert_eq!(request.method, "GET");
        assert!(request.authorized);
        assert!(request.accepts_github_json);
        assert!(request.uses_expected_api_version);
        assert!(!request.path_and_query.contains("/contents/"));
        assert!(!request.path_and_query.contains(".diff"));
        assert!(!request.path_and_query.contains(".patch"));
    }
    for path in [
        REPOSITORY_PATH,
        PULL_PATH,
        COMMITS_PATH,
        FILES_PATH,
        COMMIT_PATH,
        CHECKS_PATH,
    ] {
        assert_eq!(request_count(&requests, path), 1, "{path}");
    }
}

#[tokio::test]
async fn conditional_refresh_reports_not_modified_only_after_observing_all_subresources() {
    let server = FakeGitHubServer::start(conditional_projection_routes("success")).await;
    let previous = previous_observation();
    let result = server
        .client()
        .observe_pull_request(
            pull_request_identity(),
            Some(GitHubPullRequestCursor {
                etag: "\"pr-v1\"",
                observation: &previous,
            }),
        )
        .await
        .expect("an unchanged complete projection should be not-modified");
    assert!(matches!(result, GitHubObserveResult::NotModified));
    let requests = server.requests().await;
    assert_conditional_projection_requests(&requests);
}

#[tokio::test]
async fn conditional_refresh_observes_a_check_only_transition_after_pr_304() {
    let server = FakeGitHubServer::start(conditional_projection_routes("failure")).await;
    let previous = previous_observation();
    let result = server
        .client()
        .observe_pull_request(
            pull_request_identity(),
            Some(GitHubPullRequestCursor {
                etag: "\"pr-v1\"",
                observation: &previous,
            }),
        )
        .await
        .expect("a check-only transition should be observed");
    let GitHubObserveResult::Observed { observation, etag } = result else {
        panic!("a changed check must not be hidden by the PR 304");
    };
    assert_eq!(etag.as_deref(), Some("\"pr-v1\""));
    assert_eq!(observation.head_sha, HEAD_SHA);
    assert_eq!(observation.checks[0].conclusion.as_deref(), Some("failure"));
    assert!(!observation.has_same_provider_state(&previous));
    let requests = server.requests().await;
    assert_conditional_projection_requests(&requests);
}

#[tokio::test]
async fn conditional_refresh_uses_a_changed_pr_head_for_the_bounded_projection() {
    let server = FakeGitHubServer::start(vec![
        (
            PULL_PATH,
            vec![
                MockResponse::json(
                    StatusCode::OK,
                    &json!({
                        "title": "Compile bounded context v2",
                        "state": "open",
                        "draft": false,
                        "merged": false,
                        "base": {"sha": BASE_SHA},
                        "head": {"sha": NEW_HEAD_SHA},
                        "created_at": "2026-08-24T10:00:00Z",
                        "updated_at": "2026-08-25T11:00:00Z"
                    }),
                )
                .with_header("etag", "\"pr-v2\""),
            ],
        ),
        (
            COMMITS_PATH,
            vec![MockResponse::json(
                StatusCode::OK,
                &json!([{ "sha": NEW_COMMIT_SHA }]),
            )],
        ),
        (
            FILES_PATH,
            vec![MockResponse::json(
                StatusCode::OK,
                &json!([{ "filename": "apps/server/src/external_references.rs" }]),
            )],
        ),
        (
            NEW_CHECKS_PATH,
            vec![MockResponse::json(
                StatusCode::OK,
                &json!({
                    "check_runs": [{
                        "name": "contract",
                        "status": "in_progress",
                        "conclusion": null,
                        "details_url": "https://github.com/acme/context/actions/runs/10"
                    }]
                }),
            )],
        ),
    ])
    .await;
    let previous = previous_observation();
    let result = server
        .client()
        .observe_pull_request(
            pull_request_identity(),
            Some(GitHubPullRequestCursor {
                etag: "\"pr-v1\"",
                observation: &previous,
            }),
        )
        .await
        .expect("a changed PR should be observed from its new head");
    let GitHubObserveResult::Observed { observation, etag } = result else {
        panic!("a changed PR must produce a new observation");
    };
    assert_eq!(etag.as_deref(), Some("\"pr-v2\""));
    assert_eq!(observation.head_sha, NEW_HEAD_SHA);
    assert_eq!(observation.commits, vec![NEW_COMMIT_SHA]);
    assert_eq!(
        observation.changed_paths,
        vec!["apps/server/src/external_references.rs"]
    );

    let requests = server.requests().await;
    assert_eq!(requests.len(), 4);
    assert_eq!(request_count(&requests, PULL_PATH), 1);
    assert_eq!(request_count(&requests, COMMITS_PATH), 1);
    assert_eq!(request_count(&requests, FILES_PATH), 1);
    assert_eq!(request_count(&requests, NEW_CHECKS_PATH), 1);
    assert_eq!(request_count(&requests, CHECKS_PATH), 0);
    assert_eq!(
        requests
            .iter()
            .find(|request| request.path_and_query == PULL_PATH)
            .and_then(|request| request.if_none_match.as_deref()),
        Some("\"pr-v1\"")
    );
}

#[tokio::test]
async fn classifies_non_retryable_provider_statuses_without_leaking_bodies() {
    let server = FakeGitHubServer::start(vec![
        (
            "/status/401",
            vec![MockResponse::json(
                StatusCode::UNAUTHORIZED,
                &json!({"message": "provider-secret-401"}),
            )],
        ),
        (
            "/status/403",
            vec![MockResponse::json(
                StatusCode::FORBIDDEN,
                &json!({"message": "provider-secret-403"}),
            )],
        ),
        (
            "/status/404",
            vec![MockResponse::json(
                StatusCode::NOT_FOUND,
                &json!({"message": "provider-secret-404"}),
            )],
        ),
    ])
    .await;
    let client = server.client();
    for status in [401, 403, 404] {
        let error = client
            .get_value(&format!("/status/{status}"), &contract_token(), None)
            .await
            .expect_err("status should fail");
        let message = error.to_string();
        assert!(message.contains(&format!("status {status}")));
        assert!(!message.contains("provider-secret"));
    }
    let requests = server.requests().await;
    for status in [401, 403, 404] {
        assert_eq!(request_count(&requests, &format!("/status/{status}")), 1);
    }
}

#[tokio::test]
async fn retries_429_and_5xx_once_then_stops() {
    let retry_after = ("retry-after", "0");
    let server = FakeGitHubServer::start(vec![
        (
            "/retry/429",
            vec![
                MockResponse::json(StatusCode::TOO_MANY_REQUESTS, &json!({}))
                    .with_header(retry_after.0, retry_after.1),
                MockResponse::json(StatusCode::OK, &json!({"result": "after-429"})),
            ],
        ),
        (
            "/retry/500",
            vec![
                MockResponse::json(StatusCode::INTERNAL_SERVER_ERROR, &json!({}))
                    .with_header(retry_after.0, retry_after.1),
                MockResponse::json(StatusCode::OK, &json!({"result": "after-500"})),
            ],
        ),
        (
            "/retry/exhausted",
            vec![
                MockResponse::json(StatusCode::SERVICE_UNAVAILABLE, &json!({}))
                    .with_header(retry_after.0, retry_after.1),
                MockResponse::json(StatusCode::SERVICE_UNAVAILABLE, &json!({})),
            ],
        ),
    ])
    .await;
    let client = server.client();
    for (path, expected) in [("/retry/429", "after-429"), ("/retry/500", "after-500")] {
        let value = value_only(
            client
                .get_value(path, &contract_token(), None)
                .await
                .expect("transient status should recover"),
        )
        .expect("retry response should contain JSON");
        assert_eq!(value.get("result").and_then(Value::as_str), Some(expected));
    }
    let error = client
        .get_value("/retry/exhausted", &contract_token(), None)
        .await
        .expect_err("two failures should exhaust the bounded retry");
    assert!(error.to_string().contains("status 503"));

    let requests = server.requests().await;
    for path in ["/retry/429", "/retry/500", "/retry/exhausted"] {
        assert_eq!(request_count(&requests, path), 2, "{path}");
    }
}

#[tokio::test]
async fn enforces_timeout_and_never_follows_redirects() {
    let server = FakeGitHubServer::start(vec![
        (
            "/slow",
            vec![
                MockResponse::json(StatusCode::OK, &json!({"late": true}))
                    .delayed(Duration::from_millis(200)),
            ],
        ),
        (
            "/redirect",
            vec![
                MockResponse::json(StatusCode::FOUND, &json!(null))
                    .with_header("location", "/redirect-target"),
            ],
        ),
        (
            "/redirect-target",
            vec![MockResponse::json(
                StatusCode::OK,
                &json!({"followed": true}),
            )],
        ),
    ])
    .await;
    let timeout_error = server
        .client_with_timeout(Duration::from_millis(30))
        .get_value("/slow", &contract_token(), None)
        .await
        .expect_err("slow provider must time out");
    assert_eq!(
        timeout_error.to_string(),
        "external connector unavailable: GitHub request failed"
    );

    let redirect_error = server
        .client()
        .get_value("/redirect", &contract_token(), None)
        .await
        .expect_err("redirect must remain visible as an error");
    assert!(redirect_error.to_string().contains("status 302"));

    let requests = server.requests().await;
    assert_eq!(request_count(&requests, "/slow"), 1);
    assert_eq!(request_count(&requests, "/redirect"), 1);
    assert_eq!(request_count(&requests, "/redirect-target"), 0);
}

#[test]
fn rejects_invalid_provider_sha_before_it_can_change_a_request_path() {
    for sha in [
        "abc123",
        "../../issues",
        "222222222222222222222222222222222222222g",
        "22222222222222222222222222222222222222222",
    ] {
        assert!(
            required_git_oid(&json!({"sha": sha}), "/sha", "head SHA").is_err(),
            "{sha}"
        );
    }
    assert_eq!(
        required_git_oid(&json!({"sha": HEAD_SHA}), "/sha", "head SHA")
            .expect("full hexadecimal SHA should pass"),
        HEAD_SHA
    );
}

#[test]
fn refuses_silently_truncated_provider_pages() {
    let next = HeaderValue::from_static(
        "<https://api.github.com/repositories/1/pulls/2/files?page=2>; rel=\"next\"",
    );
    let last = HeaderValue::from_static(
        "<https://api.github.com/repositories/1/pulls/2/files?page=1>; rel=\"last\"",
    );
    assert!(has_next_page(Some(&next)));
    assert!(!has_next_page(Some(&last)));
    assert!(!has_next_page(None));
}
