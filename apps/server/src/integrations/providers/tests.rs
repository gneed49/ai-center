#![allow(clippy::needless_pass_by_value)]

use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::any,
};
use secrecy::SecretString;
use serde_json::{Value, json};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::{JoinHandle, JoinSet},
};

use super::{ApiTransport, Protocol, StructuredTransport, compile_schema, preset, validate_output};
use crate::{
    agent::{
        AgentEngine, AgentInput, ContextSelectionInput, CoverageEvaluationInput, StewardInput,
        StructuredEngine, TechnicalPlanInput,
    },
    error::{AppError, ProviderErrorClass},
};

const PROVIDERS: [&str; 5] = ["openai", "anthropic", "kimi", "deepseek", "openrouter"];

#[derive(Clone)]
struct Reply {
    status: StatusCode,
    body: String,
    delay: Duration,
    redirect: Option<String>,
}
impl Reply {
    fn json(body: Value) -> Self {
        Self {
            status: StatusCode::OK,
            body: body.to_string(),
            delay: Duration::ZERO,
            redirect: None,
        }
    }
    fn status(status: StatusCode) -> Self {
        Self {
            status,
            body: json!({"error":{"message":"private-input-and-key-must-not-leak"}}).to_string(),
            delay: Duration::ZERO,
            redirect: None,
        }
    }
}
#[derive(Clone)]
struct Captured {
    method: Method,
    uri: String,
    headers: HeaderMap,
    body: Vec<u8>,
}
#[derive(Clone)]
struct FixtureState {
    replies: Arc<Mutex<VecDeque<Reply>>>,
    requests: Arc<Mutex<Vec<Captured>>>,
    hits: Arc<AtomicUsize>,
}
struct Fixture {
    origin: String,
    state: FixtureState,
    task: JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Fixture {
    async fn start(replies: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("loopback listener");
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let state = FixtureState {
            replies: Arc::new(Mutex::new(replies.into())),
            requests: Arc::new(Mutex::new(Vec::new())),
            hits: Arc::new(AtomicUsize::new(0)),
        };
        let app = Router::new()
            .fallback(any(handler))
            .with_state(state.clone());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self {
            origin,
            state,
            task,
        }
    }
    fn client(&self, provider: &str, timeout: Duration) -> ApiTransport {
        ApiTransport::new_for_local_development(
            provider,
            SecretString::from("fixture-credential-only"),
            &format!("{}{}", self.origin, preset(provider).unwrap().generate_path),
            timeout,
            Duration::from_millis(1),
        )
        .unwrap()
    }
}
async fn handler(
    State(state): State<FixtureState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    state.hits.fetch_add(1, Ordering::SeqCst);
    state.requests.lock().unwrap().push(Captured {
        method,
        uri: uri.to_string(),
        headers,
        body: body.to_vec(),
    });
    let reply = {
        let mut replies = state.replies.lock().unwrap();
        if replies.len() > 1 {
            replies.pop_front().unwrap()
        } else {
            replies.front().unwrap().clone()
        }
    };
    tokio::time::sleep(reply.delay).await;
    let mut response = (reply.status, reply.body).into_response();
    response
        .headers_mut()
        .insert("x-request-id", "request-fixture".parse().unwrap());
    if let Some(location) = reply.redirect {
        response
            .headers_mut()
            .insert("location", location.parse().unwrap());
    }
    response
}
fn schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["ok"],"properties":{"ok":{"type":"boolean"}}})
}
fn response(provider: &str, output: Value) -> Value {
    let text = output.to_string();
    match preset(provider).unwrap().protocol {
        Protocol::Responses => {
            json!({"id":"resp-fixture","model":"model-served","status":"completed","output":[{"type":"reasoning"},{"type":"message","status":"completed","content":[{"type":"output_text","text":text}]}],"usage":{"input_tokens":13,"output_tokens":8}})
        }
        Protocol::Anthropic => {
            json!({"id":"msg-fixture","model":"model-served","stop_reason":"end_turn","content":[{"type":"text","text":text}],"usage":{"input_tokens":13,"output_tokens":8}})
        }
        _ => {
            json!({"id":"chat-fixture","model":"model-served","choices":[{"finish_reason":"stop","message":{"role":"assistant","content":text}}],"usage":{"prompt_tokens":13,"completion_tokens":8}})
        }
    }
}
async fn generate(client: &ApiTransport) -> Result<super::StructuredResponse, AppError> {
    client
        .generate(
            "model-requested",
            "fixture_contract",
            "Preserve the business instructions",
            &json!({"context":"private-fixture"}),
            &schema(),
        )
        .await
}
fn assert_error(error: &AppError, class: ProviderErrorClass, attempts: u32) {
    let AppError::Provider(error) = error else {
        panic!("expected sanitized provider error: {error}");
    };
    assert_eq!(error.class(), class);
    assert_eq!(error.attempts(), attempts);
    assert!(!error.to_string().contains("private-input"));
    assert!(!error.to_string().contains("fixture-credential"));
}

#[tokio::test]
async fn official_protocols_preserve_context_schema_model_and_usage_without_tools() {
    for provider in PROVIDERS {
        let fixture =
            Fixture::start(vec![Reply::json(response(provider, json!({"ok":true})))]).await;
        let output = generate(&fixture.client(provider, Duration::from_secs(1)))
            .await
            .unwrap();
        assert_eq!(output.output, json!({"ok":true}));
        assert_eq!(output.metadata.provider, provider);
        assert_eq!(output.metadata.requested_model, "model-requested");
        assert_eq!(
            output.metadata.served_model.as_deref(),
            Some("model-served")
        );
        assert_eq!(
            output.metadata.provider_request_id.as_deref(),
            Some("request-fixture")
        );
        assert_eq!(output.metadata.input_tokens, Some(13));
        assert_eq!(output.metadata.output_tokens, Some(8));
        assert_eq!(output.metadata.attempts, 1);
        let requests = fixture.state.requests.lock().unwrap();
        let request = &requests[0];
        assert_eq!(request.method, Method::POST);
        assert_eq!(request.uri, preset(provider).unwrap().generate_path);
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["model"], "model-requested");
        assert_eq!(body["stream"], false);
        assert!(body.get("tools").is_none());
        assert!(body.get("models").is_none());
        if provider == "anthropic" {
            assert_eq!(request.headers["x-api-key"], "fixture-credential-only");
            assert_eq!(request.headers["anthropic-version"], "2023-06-01");
            assert!(request.headers.get("authorization").is_none());
            assert_eq!(body["system"], "Preserve the business instructions");
            assert_eq!(body["output_config"]["format"]["schema"], schema());
        } else {
            assert_eq!(
                request.headers["authorization"],
                "Bearer fixture-credential-only"
            );
        }
        match preset(provider).unwrap().protocol {
            Protocol::Responses => {
                assert_eq!(body["instructions"], "Preserve the business instructions");
                assert_eq!(body["text"]["format"]["schema"], schema());
                assert_eq!(
                    body["input"][0]["content"][0]["text"],
                    json!({"context":"private-fixture"}).to_string()
                );
            }
            Protocol::ChatJson => {
                assert_eq!(body["response_format"], json!({"type":"json_object"}));
                assert_eq!(body["max_completion_tokens"], 8192);
                assert!(body.get("max_tokens").is_none());
                assert!(
                    body["messages"][0]["content"]
                        .as_str()
                        .unwrap()
                        .contains(&schema().to_string())
                );
            }
            Protocol::ChatSchema => {
                assert_eq!(body["response_format"]["json_schema"]["schema"], schema());
                assert_eq!(
                    body["provider"],
                    json!({"require_parameters":true,"allow_fallbacks":false})
                );
            }
            Protocol::Anthropic => {}
        }
        if provider == "openai" {
            assert_eq!(body["store"], false);
            assert_eq!(body["text"]["format"]["strict"], true);
        }
        if provider == "deepseek" {
            assert!(body.get("store").is_none());
            assert!(body["text"]["format"].get("strict").is_none());
        }
    }
}

#[tokio::test]
async fn invalid_json_schema_refusal_and_truncation_are_rejected_for_every_protocol() {
    for provider in PROVIDERS {
        let mut incomplete = response(provider, json!({"ok":true}));
        let mut refusal = incomplete.clone();
        match preset(provider).unwrap().protocol {
            Protocol::Responses => {
                incomplete["status"] = json!("incomplete");
                refusal["output"][1]["content"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"type":"refusal","refusal":"private input"}));
            }
            Protocol::Anthropic => {
                incomplete["stop_reason"] = json!("max_tokens");
                refusal["stop_reason"] = json!("refusal");
            }
            _ => {
                incomplete["choices"][0]["finish_reason"] = json!("length");
                refusal["choices"][0]["message"]["refusal"] = json!("private input");
            }
        }
        for reply in [
            Reply::json(incomplete),
            Reply::json(refusal),
            Reply::json(response(provider, json!({"ok":"wrong-type"}))),
            Reply::json(response(provider, json!({"ok":true,"extra":"private"}))),
            Reply {
                body: "private-invalid-json".into(),
                ..Reply::json(json!({}))
            },
        ] {
            let fixture = Fixture::start(vec![reply]).await;
            assert_error(
                &generate(&fixture.client(provider, Duration::from_secs(1)))
                    .await
                    .unwrap_err(),
                ProviderErrorClass::ResponseContract,
                1,
            );
            assert_eq!(fixture.state.hits.load(Ordering::SeqCst), 1);
        }
    }
}

#[tokio::test]
async fn retries_are_bounded_and_never_switch_provider_or_credential() {
    for provider in PROVIDERS {
        for (status, class, attempts) in [
            (
                StatusCode::UNAUTHORIZED,
                ProviderErrorClass::Authentication,
                1,
            ),
            (
                StatusCode::TOO_MANY_REQUESTS,
                ProviderErrorClass::RateLimit,
                3,
            ),
            (
                StatusCode::SERVICE_UNAVAILABLE,
                ProviderErrorClass::Server,
                3,
            ),
        ] {
            let fixture = Fixture::start(vec![Reply::status(status)]).await;
            assert_error(
                &generate(&fixture.client(provider, Duration::from_secs(1)))
                    .await
                    .unwrap_err(),
                class,
                attempts,
            );
            assert_eq!(fixture.state.hits.load(Ordering::SeqCst), attempts as usize);
            assert!(
                fixture
                    .state
                    .requests
                    .lock()
                    .unwrap()
                    .windows(2)
                    .all(|requests| requests[0].headers == requests[1].headers
                        && requests[0].body == requests[1].body
                        && requests[0].uri == requests[1].uri)
            );
        }
        let fixture = Fixture::start(vec![
            Reply::status(StatusCode::TOO_MANY_REQUESTS),
            Reply::json(response(provider, json!({"ok":true}))),
        ])
        .await;
        assert_eq!(
            generate(&fixture.client(provider, Duration::from_secs(1)))
                .await
                .unwrap()
                .metadata
                .attempts,
            2
        );
    }
}

#[tokio::test]
async fn catalogue_get_is_authenticated_and_sends_no_prompt_or_generation() {
    for provider in PROVIDERS {
        let fixture = Fixture::start(vec![Reply::json(json!({"data":[{"id":"model-b","name":"Model B"},{"id":"model-a","display_name":"Model A"}],"has_more":false}))]).await;
        let models = fixture
            .client(provider, Duration::from_secs(1))
            .list_models()
            .await
            .unwrap();
        assert_eq!(
            models
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>(),
            ["model-a", "model-b"]
        );
        let requests = fixture.state.requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, Method::GET);
        assert!(requests[0].body.is_empty());
        assert!(
            requests[0]
                .uri
                .starts_with(preset(provider).unwrap().models_path)
        );
        assert!(
            requests[0]
                .headers
                .contains_key(if provider == "anthropic" {
                    "x-api-key"
                } else {
                    "authorization"
                })
        );
    }
}

#[tokio::test]
async fn anthropic_catalogue_paginates_by_cursor_on_the_same_origin_and_is_bounded() {
    let fixture = Fixture::start(vec![
        Reply::json(json!({"data":[{"id":"model-a"}],"has_more":true,"last_id":"model-a"})),
        Reply::json(json!({"data":[{"id":"model-b"}],"has_more":false})),
    ])
    .await;
    let models = fixture
        .client("anthropic", Duration::from_secs(1))
        .list_models()
        .await
        .unwrap();
    assert_eq!(models.len(), 2);
    assert_eq!(
        fixture.state.requests.lock().unwrap()[1].uri,
        "/v1/models?limit=1000&after_id=model-a"
    );
    let fixture = Fixture::start(vec![Reply::json(
        json!({"data":[{"id":"model-a"}],"has_more":true,"last_id":"model-a"}),
    )])
    .await;
    assert_error(
        &fixture
            .client("anthropic", Duration::from_secs(1))
            .list_models()
            .await
            .unwrap_err(),
        ProviderErrorClass::ResponseContract,
        1,
    );
    assert_eq!(fixture.state.hits.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn redirects_are_not_followed_even_to_another_loopback_origin() {
    let target = Fixture::start(vec![Reply::json(json!({}))]).await;
    for provider in PROVIDERS {
        let fixture = Fixture::start(vec![Reply {
            redirect: Some(target.origin.clone()),
            ..Reply::status(StatusCode::TEMPORARY_REDIRECT)
        }])
        .await;
        assert_error(
            &generate(&fixture.client(provider, Duration::from_secs(1)))
                .await
                .unwrap_err(),
            ProviderErrorClass::Request,
            1,
        );
        assert_eq!(target.state.hits.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn deadlines_and_cancellation_stop_local_retry_work_for_every_provider() {
    for provider in PROVIDERS {
        let fixture = Fixture::start(vec![Reply {
            delay: Duration::from_secs(1),
            ..Reply::json(response(provider, json!({"ok":true})))
        }])
        .await;
        assert_error(
            &generate(&fixture.client(provider, Duration::from_millis(10)))
                .await
                .unwrap_err(),
            ProviderErrorClass::Timeout,
            3,
        );
        let fixture = Fixture::start(vec![Reply {
            delay: Duration::from_secs(1),
            ..Reply::json(response(provider, json!({"ok":true})))
        }])
        .await;
        let client = fixture.client(provider, Duration::from_secs(1));
        let task = tokio::spawn(async move { generate(&client).await });
        tokio::time::timeout(Duration::from_secs(1), async {
            while fixture.state.hits.load(Ordering::SeqCst) == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(fixture.state.hits.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn oversized_responses_are_rejected_without_retaining_provider_data() {
    let fixture = Fixture::start(vec![Reply {
        body: "X".repeat(super::MAX_BODY_BYTES + 1),
        ..Reply::json(json!({}))
    }])
    .await;
    assert_error(
        &generate(&fixture.client("kimi", Duration::from_secs(1)))
            .await
            .unwrap_err(),
        ProviderErrorClass::ResponseContract,
        1,
    );
}

#[test]
fn schemas_validate_uuid_enums_bounds_uniqueness_and_never_fetch_remote_references() {
    let schema = json!({"type":"object","additionalProperties":false,"required":["ids","confidence","verdict"],"properties":{"ids":{"type":"array","uniqueItems":true,"minItems":1,"items":{"type":"string","format":"uuid"}},"confidence":{"type":"number","minimum":0,"maximum":1},"verdict":{"enum":["planned","unaddressed"]}}});
    let valid = json!({"ids":["69a0b9d8-99bf-4e4c-b144-3a68d3ea474d"],"confidence":0.5,"verdict":"planned"});
    validate_output("kimi", &schema, &valid).unwrap();
    for (field, value) in [
        ("ids", json!(["not-a-uuid"])),
        ("ids", json!([])),
        (
            "ids",
            json!([
                "69a0b9d8-99bf-4e4c-b144-3a68d3ea474d",
                "69a0b9d8-99bf-4e4c-b144-3a68d3ea474d"
            ]),
        ),
        ("confidence", json!(1.1)),
        ("verdict", json!("invented")),
    ] {
        let mut invalid = valid.clone();
        invalid[field] = value;
        assert!(validate_output("kimi", &schema, &invalid).is_err());
    }
    for reference in [
        "https://example.com/private-schema",
        "file:///etc/passwd",
        "http://127.0.0.1:1/private-schema",
    ] {
        assert!(compile_schema(&json!({"$ref":reference})).is_err());
    }
}

#[test]
fn presets_are_fixed_and_loopback_override_rejects_ssrf_or_ambiguous_urls() {
    for provider in PROVIDERS {
        let preset = preset(provider).unwrap();
        let client = ApiTransport::new(provider, SecretString::from("fixture-key")).unwrap();
        assert_eq!(client.origin.as_str().trim_end_matches('/'), preset.origin);
        for origin in [
            "https://example.com",
            "http://localhost.evil.test",
            "http://user:password@127.0.0.1",
            "file://",
        ] {
            assert!(
                ApiTransport::new_for_local_development(
                    provider,
                    SecretString::from("fixture-key"),
                    &format!("{origin}{}", preset.generate_path),
                    Duration::from_secs(1),
                    Duration::ZERO
                )
                .is_err()
            );
        }
        assert!(
            ApiTransport::new_for_local_development(
                provider,
                SecretString::from("fixture-key"),
                &format!("http://127.0.0.1{}?target=evil", preset.generate_path),
                Duration::from_secs(1),
                Duration::ZERO
            )
            .is_err()
        );
    }
    assert!(ApiTransport::new("arbitrary-provider", SecretString::from("fixture-key")).is_err());
}

#[tokio::test]
async fn all_five_business_operations_share_the_engine_across_every_transport() {
    for provider in PROVIDERS {
        let plan = json!({"title":"Plan","summary":"Summary","architecture":"Architecture","delivery_slices":[],"risks":[],"validation":[],"coverage":[]});
        let outputs = [
            json!({"response":"Answer","proposals":[],"sources":[]}),
            json!({"selected_version_ids":[]}),
            plan,
            json!({"requirements":[]}),
            json!({"assessments":[]}),
        ];
        let fixture = Fixture::start(
            outputs
                .into_iter()
                .map(|output| Reply::json(response(provider, output)))
                .collect(),
        )
        .await;
        let engine = StructuredEngine::with_transport(
            Arc::new(fixture.client(provider, Duration::from_secs(1))),
            "requested-model".into(),
        );
        let turn = engine
            .respond(AgentInput {
                scope_kind: "project".into(),
                instructions: "Business instruction".into(),
                user_message: "User objective".into(),
                context: json!({}),
            })
            .await
            .unwrap();
        assert_eq!(turn.metadata.provider, provider);
        engine
            .select_context(ContextSelectionInput {
                objective: "Objective".into(),
                task_kind: "implementation".into(),
                candidates: json!([]),
            })
            .await
            .unwrap();
        let plan = engine
            .generate_technical_plan(TechnicalPlanInput {
                objective: "Objective".into(),
                context_pack: json!({}),
            })
            .await
            .unwrap();
        engine
            .evaluate_coverage(CoverageEvaluationInput {
                context_pack: json!({}),
                technical_plan: plan.output,
            })
            .await
            .unwrap();
        engine
            .analyze_contradictions(StewardInput {
                candidate_pairs: json!([]),
            })
            .await
            .unwrap();
        assert_eq!(fixture.state.hits.load(Ordering::SeqCst), 5);
    }
}

#[tokio::test]
async fn anthropic_wire_schema_adapts_limits_but_server_enforces_original_contract() {
    let schema = json!({"type":"object","additionalProperties":false,"required":["confidence","ids"],"properties":{"confidence":{"type":"number","minimum":0,"maximum":1},"ids":{"type":"array","minItems":2,"maxItems":2,"uniqueItems":true,"items":{"type":"string","format":"uuid"}}}});
    let invalid = json!({"confidence":2,"ids":[]});
    let fixture = Fixture::start(vec![Reply::json(response("anthropic", invalid))]).await;
    let client = fixture.client("anthropic", Duration::from_secs(1));
    assert_error(
        &client
            .generate(
                "model-requested",
                "fixture",
                "instructions",
                &json!({}),
                &schema,
            )
            .await
            .unwrap_err(),
        ProviderErrorClass::ResponseContract,
        1,
    );
    let requests = fixture.state.requests.lock().unwrap();
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    let sent = &body["output_config"]["format"]["schema"];
    assert!(sent["properties"]["confidence"].get("minimum").is_none());
    assert!(
        sent["properties"]["confidence"]["description"]
            .as_str()
            .unwrap()
            .contains("maximum: 1")
    );
    assert_eq!(sent["properties"]["ids"]["minItems"], 1);
    assert!(sent["properties"]["ids"].get("uniqueItems").is_none());
    assert!(schema["properties"]["ids"].get("uniqueItems").is_some());
    assert_eq!(sent["properties"]["ids"]["items"]["format"], "uuid");
}

#[tokio::test]
async fn catalogue_rejects_bad_contracts_and_has_one_total_deadline() {
    for provider in PROVIDERS {
        let fixture = Fixture::start(vec![Reply::json(json!({"data":[{"id":"bad model"}]}))]).await;
        assert_error(
            &fixture
                .client(provider, Duration::from_secs(1))
                .list_models()
                .await
                .unwrap_err(),
            ProviderErrorClass::ResponseContract,
            1,
        );
        let fixture = Fixture::start(vec![Reply {
            delay: Duration::from_millis(200),
            ..Reply::json(json!({"data":[],"has_more":false}))
        }])
        .await;
        assert_error(
            &fixture
                .client(provider, Duration::from_millis(20))
                .list_models()
                .await
                .unwrap_err(),
            ProviderErrorClass::Timeout,
            1,
        );
        assert_eq!(fixture.state.hits.load(Ordering::SeqCst), 1);
    }
}

struct UnvalidatedTransport {
    output: Value,
}
#[async_trait::async_trait]
impl StructuredTransport for UnvalidatedTransport {
    fn provider_name(&self) -> &'static str {
        "fixture-subscription"
    }
    async fn generate(
        &self,
        _model: &str,
        _operation: &str,
        _instructions: &str,
        _input: &Value,
        _schema: &Value,
    ) -> crate::error::AppResult<super::StructuredResponse> {
        Ok(super::StructuredResponse {
            output: self.output.clone(),
            metadata: crate::agent::AgentRunMetadata {
                provider: "different-provider".into(),
                requested_model: "different-model".into(),
                ..Default::default()
            },
        })
    }
    async fn list_models(&self) -> crate::error::AppResult<Vec<super::ProviderModel>> {
        Ok(Vec::new())
    }
}

#[tokio::test]
async fn engine_validates_non_http_transports_and_pins_selected_identity() {
    let input = || ContextSelectionInput {
        objective: "Objective".into(),
        task_kind: "implementation".into(),
        candidates: json!([]),
    };
    let bad = StructuredEngine::with_transport(
        Arc::new(UnvalidatedTransport {
            output: json!({"selected_version_ids":[],"extra":true}),
        }),
        "selected-model".into(),
    );
    assert_error(
        &bad.select_context(input()).await.unwrap_err(),
        ProviderErrorClass::ResponseContract,
        1,
    );
    let valid = StructuredEngine::with_transport(
        Arc::new(UnvalidatedTransport {
            output: json!({"selected_version_ids":[]}),
        }),
        "selected-model".into(),
    );
    let output = valid.select_context(input()).await.unwrap();
    assert_eq!(output.metadata.provider, "fixture-subscription");
    assert_eq!(output.metadata.requested_model, "selected-model");
}

// Send valid headers and only part of the announced body before stalling or
// closing. Unlike a delayed handler, this exercises response.chunk() failures.
async fn body_failure_fixture(stall: bool, recover: bool) -> Fixture {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let state = FixtureState {
        replies: Arc::new(Mutex::new(VecDeque::new())),
        requests: Arc::new(Mutex::new(Vec::new())),
        hits: Arc::new(AtomicUsize::new(0)),
    };
    let hits = Arc::clone(&state.hits);
    let task = tokio::spawn(async move {
        let mut connections = JoinSet::new();
        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let (mut stream, _) = accepted.unwrap();
                    let hits = Arc::clone(&hits);
                    connections.spawn(async move {
                        let mut request = Vec::new();
                        let mut buffer = [0_u8; 4096];
                        loop {
                            let read = stream.read(&mut buffer).await.unwrap();
                            if read == 0 { return; }
                            request.extend_from_slice(&buffer[..read]);
                            if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                                let headers = String::from_utf8_lossy(&request[..end]);
                                let length = headers.lines().find_map(|line| {
                                    let (key, value) = line.split_once(':')?;
                                    key.eq_ignore_ascii_case("content-length").then(|| value.trim().parse::<usize>().unwrap())
                                }).unwrap_or(0);
                                if request.len() >= end + 4 + length { break; }
                            }
                        }
                        let attempt = hits.fetch_add(1, Ordering::SeqCst) + 1;
                        if recover && attempt > 1 {
                            let body = response("openai", json!({"ok":true})).to_string();
                            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
                            stream.write_all(response.as_bytes()).await.unwrap();
                        } else {
                            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 200\r\nConnection: close\r\n\r\n{").await.unwrap();
                            if stall { tokio::time::sleep(Duration::from_secs(2)).await; }
                        }
                    });
                }
                _ = connections.join_next(), if !connections.is_empty() => {}
            }
        }
    });
    Fixture {
        origin,
        state,
        task,
    }
}

#[tokio::test]
async fn body_timeouts_and_disconnections_retry_without_becoming_contract_errors() {
    for (stall, class) in [
        (true, ProviderErrorClass::Timeout),
        (false, ProviderErrorClass::Connection),
    ] {
        let fixture = body_failure_fixture(stall, false).await;
        let error = generate(&fixture.client("openai", Duration::from_millis(100)))
            .await
            .unwrap_err();
        assert_error(&error, class, 3);
        assert!(error.is_retryable_provider_failure());
        assert_eq!(fixture.state.hits.load(Ordering::SeqCst), 3);

        let fixture = body_failure_fixture(stall, true).await;
        let result = generate(&fixture.client("openai", Duration::from_millis(100)))
            .await
            .unwrap();
        assert_eq!(result.metadata.attempts, 2);
        assert_eq!(result.output, json!({"ok":true}));
        assert_eq!(fixture.state.hits.load(Ordering::SeqCst), 2);
    }
}
