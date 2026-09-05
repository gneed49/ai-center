use std::time::{Duration, Instant};

use reqwest::{Client, StatusCode, redirect::Policy};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use tokio::time::sleep;
#[cfg(debug_assertions)]
use url::Host;
use url::Url;

use crate::error::{AppError, AppResult, ProviderError, ProviderErrorClass};

const RESPONSES_ENDPOINT: &str = "https://api.openai.com/v1/responses";
const MAX_ATTEMPTS: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAiResponseMetadata {
    pub served_model: Option<String>,
    pub provider_response_id: Option<String>,
    pub provider_request_id: Option<String>,
    pub status: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub latency_ms: i64,
    pub attempts: u32,
}

#[derive(Debug, Clone)]
pub struct OpenAiStructuredResponse<T> {
    pub output: T,
    pub metadata: OpenAiResponseMetadata,
}

#[derive(Debug, Clone, Copy)]
struct RetryPolicy {
    max_attempts: u32,
    base_delay: Duration,
}

/// Minimal Responses API client with a fixed production origin.
///
/// Production construction never reads an endpoint from configuration. The
/// only endpoint override is a loopback-only constructor that is unavailable
/// in release builds, so an attacker-controlled URL cannot turn provider calls
/// into SSRF.
pub struct OpenAiResponsesClient {
    client: Client,
    api_key: SecretString,
    endpoint: Url,
    retry: RetryPolicy,
}

impl OpenAiResponsesClient {
    /// Build a production client pinned to the official Responses API.
    ///
    /// # Errors
    ///
    /// Returns an internal configuration error if the hardened HTTP client
    /// cannot be constructed.
    pub fn new(api_key: SecretString) -> AppResult<Self> {
        let endpoint = Url::parse(RESPONSES_ENDPOINT)
            .map_err(|_| AppError::Internal("invalid built-in OpenAI endpoint".into()))?;
        Self::build(
            api_key,
            endpoint,
            Duration::from_secs(10),
            Duration::from_secs(60),
            RetryPolicy {
                max_attempts: MAX_ATTEMPTS,
                base_delay: Duration::from_millis(250),
            },
        )
    }

    /// Build a loopback-only client for explicit local development and HTTP
    /// contract tests. This entry point does not exist in release builds.
    ///
    /// # Errors
    ///
    /// Rejects non-loopback hosts, credentials, query strings, fragments, and
    /// any path other than the Responses API path.
    #[cfg(debug_assertions)]
    pub fn new_for_local_development(
        api_key: SecretString,
        endpoint: &str,
        timeout: Duration,
        retry_delay: Duration,
    ) -> AppResult<Self> {
        let endpoint = validate_loopback_endpoint(endpoint)?;
        Self::build(
            api_key,
            endpoint,
            timeout,
            timeout,
            RetryPolicy {
                max_attempts: MAX_ATTEMPTS,
                base_delay: retry_delay,
            },
        )
    }

    fn build(
        api_key: SecretString,
        endpoint: Url,
        connect_timeout: Duration,
        timeout: Duration,
        retry: RetryPolicy,
    ) -> AppResult<Self> {
        let client = Client::builder()
            .connect_timeout(connect_timeout)
            .timeout(timeout)
            .redirect(Policy::none())
            .build()
            .map_err(|_| AppError::Internal("failed to build OpenAI HTTP client".into()))?;
        Ok(Self {
            client,
            api_key,
            endpoint,
            retry,
        })
    }

    /// Send one Structured Outputs request and deserialize its `output_text`.
    ///
    /// Transport timeouts, connection failures, HTTP 429, and 5xx responses
    /// are retried at most twice. Authentication, authorization, and contract
    /// errors are returned immediately. Provider bodies are never copied into
    /// errors, avoiding accidental disclosure of submitted context.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Provider`] after classifying an exhausted transport
    /// or status failure, an unreadable API response, missing output, or output
    /// that does not match the expected Rust type.
    pub async fn request_structured<T: DeserializeOwned>(
        &self,
        model: &str,
        operation_name: &str,
        instructions: &str,
        input: &Value,
        schema: &Value,
    ) -> AppResult<OpenAiStructuredResponse<T>> {
        let body = structured_request_body(model, operation_name, instructions, input, schema)?;
        let started = Instant::now();
        let mut attempts = 0_u32;

        loop {
            attempts += 1;
            let response = self
                .client
                .post(self.endpoint.clone())
                .bearer_auth(self.api_key.expose_secret())
                .json(&body)
                .send()
                .await;

            let response = match response {
                Ok(response) => response,
                Err(error)
                    if attempts < self.retry.max_attempts
                        && (error.is_timeout() || error.is_connect()) =>
                {
                    self.wait_before_retry(attempts).await;
                    continue;
                }
                Err(error) => {
                    let class = if error.is_timeout() {
                        ProviderErrorClass::Timeout
                    } else if error.is_connect() {
                        ProviderErrorClass::Connection
                    } else {
                        ProviderErrorClass::Transport
                    };
                    return Err(provider_error(class, attempts, None));
                }
            };

            let status = response.status();
            let request_id = response
                .headers()
                .get("x-request-id")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            if status == StatusCode::TOO_MANY_REQUESTS {
                let class = classify_rate_limit_response(response).await;
                if class.is_retryable() && attempts < self.retry.max_attempts {
                    self.wait_before_retry(attempts).await;
                    continue;
                }
                return Err(provider_error(class, attempts, Some(status.as_u16())));
            }
            if attempts < self.retry.max_attempts && should_retry_status(status) {
                self.wait_before_retry(attempts).await;
                continue;
            }
            if !status.is_success() {
                return Err(provider_error(
                    classify_status(status),
                    attempts,
                    Some(status.as_u16()),
                ));
            }

            let value: Value = response.json().await.map_err(|_| {
                provider_error(ProviderErrorClass::ResponseContract, attempts, None)
            })?;
            return parse_structured_response(&value, request_id, attempts, started.elapsed());
        }
    }

    async fn wait_before_retry(&self, attempt: u32) {
        sleep(self.retry.base_delay.saturating_mul(attempt)).await;
    }
}

fn should_retry_status(status: StatusCode) -> bool {
    status.is_server_error()
}

#[derive(Deserialize)]
struct OpenAiErrorEnvelope {
    error: Option<OpenAiErrorDescriptor>,
}

#[derive(Deserialize)]
struct OpenAiErrorDescriptor {
    code: Option<String>,
    #[serde(rename = "type")]
    error_type: Option<String>,
}

async fn classify_rate_limit_response(response: reqwest::Response) -> ProviderErrorClass {
    let descriptor = response
        .json::<OpenAiErrorEnvelope>()
        .await
        .ok()
        .and_then(|envelope| envelope.error);
    if descriptor.as_ref().is_some_and(|error| {
        error.code.as_deref() == Some("insufficient_quota")
            || error.error_type.as_deref() == Some("insufficient_quota")
    }) {
        ProviderErrorClass::Quota
    } else {
        ProviderErrorClass::RateLimit
    }
}

fn classify_status(status: StatusCode) -> ProviderErrorClass {
    match status {
        StatusCode::UNAUTHORIZED => ProviderErrorClass::Authentication,
        StatusCode::FORBIDDEN => ProviderErrorClass::Permission,
        StatusCode::NOT_FOUND => ProviderErrorClass::NotFound,
        StatusCode::TOO_MANY_REQUESTS => ProviderErrorClass::RateLimit,
        status if status.is_server_error() => ProviderErrorClass::Server,
        _ => ProviderErrorClass::Request,
    }
}

fn provider_error(
    class: ProviderErrorClass,
    attempts: u32,
    upstream_status: Option<u16>,
) -> AppError {
    ProviderError::new("OpenAI", class, attempts, upstream_status).into()
}

fn structured_request_body(
    model: &str,
    operation_name: &str,
    instructions: &str,
    input: &Value,
    schema: &Value,
) -> AppResult<Value> {
    Ok(json!({
        "model": model,
        "store": false,
        "instructions": instructions,
        "input": [{
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": serde_json::to_string(input)
                    .map_err(|error| AppError::Internal(error.to_string()))?
            }]
        }],
        "text": {
            "format": {
                "type": "json_schema",
                "name": operation_name,
                "strict": true,
                "schema": schema
            }
        }
    }))
}

fn parse_structured_response<T: DeserializeOwned>(
    value: &Value,
    provider_request_id: Option<String>,
    attempts: u32,
    elapsed: Duration,
) -> AppResult<OpenAiStructuredResponse<T>> {
    let output_text = structured_output_text(value)
        .ok_or_else(|| provider_error(ProviderErrorClass::ResponseContract, attempts, None))?;
    let output = serde_json::from_str(output_text)
        .map_err(|_| provider_error(ProviderErrorClass::ResponseContract, attempts, None))?;
    Ok(OpenAiStructuredResponse {
        output,
        metadata: OpenAiResponseMetadata {
            served_model: value
                .get("model")
                .and_then(Value::as_str)
                .map(str::to_owned),
            provider_response_id: value.get("id").and_then(Value::as_str).map(str::to_owned),
            provider_request_id,
            status: value
                .get("status")
                .and_then(Value::as_str)
                .map(str::to_owned),
            input_tokens: value.pointer("/usage/input_tokens").and_then(Value::as_i64),
            output_tokens: value
                .pointer("/usage/output_tokens")
                .and_then(Value::as_i64),
            latency_ms: i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX),
            attempts,
        },
    })
}

fn structured_output_text(value: &Value) -> Option<&str> {
    value
        .get("output")
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .find(|item| item.get("type").and_then(Value::as_str) == Some("message"))
        })
        .and_then(|item| item.get("content"))
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .find(|item| item.get("type").and_then(Value::as_str) == Some("output_text"))
        })
        .and_then(|item| item.get("text"))
        .and_then(Value::as_str)
}

#[cfg(debug_assertions)]
fn validate_loopback_endpoint(endpoint: &str) -> AppResult<Url> {
    let url = Url::parse(endpoint)
        .map_err(|_| AppError::Invalid("invalid local OpenAI endpoint".into()))?;
    let loopback = match url.host() {
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        Some(Host::Domain(domain)) => domain == "localhost",
        None => false,
    };
    let valid = matches!(url.scheme(), "http" | "https")
        && loopback
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && url.path() == "/v1/responses";
    if !valid {
        return Err(AppError::Invalid(
            "local OpenAI endpoint must be an exact loopback Responses API URL".into(),
        ));
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use axum::{
        Json, Router,
        body::Bytes,
        extract::State,
        http::{HeaderValue, StatusCode},
        response::{IntoResponse, Response},
        routing::post,
    };
    use secrecy::SecretString;
    use serde::Deserialize;
    use serde_json::{Value, json};
    use tokio::{net::TcpListener, task::JoinHandle, time::sleep};

    use super::OpenAiResponsesClient;
    use crate::error::{AppError, ProviderErrorClass};

    #[derive(Clone, Copy)]
    enum FixtureMode {
        Success,
        Status(StatusCode),
        RateLimit,
        RateLimitThenSuccess,
        InsufficientQuota,
        Slow,
        InvalidApiJson,
        MissingStructuredOutput,
        InvalidStructuredOutput,
        OperationContracts,
    }

    #[derive(Clone)]
    struct FixtureState {
        mode: FixtureMode,
        hits: Arc<AtomicUsize>,
        bodies: Arc<Mutex<Vec<Value>>>,
    }

    struct FixtureServer {
        endpoint: String,
        hits: Arc<AtomicUsize>,
        bodies: Arc<Mutex<Vec<Value>>>,
        task: JoinHandle<()>,
    }

    impl Drop for FixtureServer {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields)]
    struct FixtureOutput {
        response: String,
    }

    async fn start_fixture(mode: FixtureMode) -> FixtureServer {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("fixture listener should bind");
        let address = listener
            .local_addr()
            .expect("fixture should have an address");
        let hits = Arc::new(AtomicUsize::new(0));
        let bodies = Arc::new(Mutex::new(Vec::new()));
        let state = FixtureState {
            mode,
            hits: Arc::clone(&hits),
            bodies: Arc::clone(&bodies),
        };
        let app = Router::new()
            .route("/v1/responses", post(fixture_handler))
            .with_state(state);
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("fixture server should stay available");
        });
        FixtureServer {
            endpoint: format!("http://{address}/v1/responses"),
            hits,
            bodies,
            task,
        }
    }

    async fn fixture_handler(State(state): State<FixtureState>, body: Bytes) -> Response {
        let hit = state.hits.fetch_add(1, Ordering::SeqCst) + 1;
        let parsed_body = serde_json::from_slice::<Value>(&body).ok();
        let operation_name = parsed_body
            .as_ref()
            .and_then(|body| body.pointer("/text/format/name"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        if let Some(body) = parsed_body {
            state
                .bodies
                .lock()
                .expect("fixture body lock should not be poisoned")
                .push(body);
        }
        match state.mode {
            FixtureMode::Status(status) => status.into_response(),
            FixtureMode::RateLimitThenSuccess if hit < 3 => provider_error_response(
                "rate_limit_exceeded",
                "rate_limit_error",
                "temporary fixture rate limit",
            ),
            FixtureMode::Success | FixtureMode::RateLimitThenSuccess => success_response(),
            FixtureMode::RateLimit => provider_error_response(
                "rate_limit_exceeded",
                "rate_limit_error",
                "temporary fixture rate limit",
            ),
            FixtureMode::InsufficientQuota => provider_error_response(
                "insufficient_quota",
                "insufficient_quota",
                "sensitive fixture billing detail must never be retained",
            ),
            FixtureMode::Slow => {
                sleep(Duration::from_millis(100)).await;
                success_response()
            }
            FixtureMode::InvalidApiJson => (StatusCode::OK, "not-json").into_response(),
            FixtureMode::MissingStructuredOutput => Json(json!({
                "id": "resp_contract_missing",
                "status": "completed",
                "model": "gpt-contract",
                "output": []
            }))
            .into_response(),
            FixtureMode::InvalidStructuredOutput => Json(json!({
                "id": "resp_contract_invalid",
                "status": "completed",
                "model": "gpt-contract",
                "output": [{
                    "type": "message",
                    "content": [{"type": "output_text", "text": "{\"unexpected\":true}"}]
                }]
            }))
            .into_response(),
            FixtureMode::OperationContracts => operation_response(operation_name.as_deref()),
        }
    }

    fn provider_error_response(code: &str, error_type: &str, message: &str) -> Response {
        (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": {
                    "code": code,
                    "type": error_type,
                    "message": message
                }
            })),
        )
            .into_response()
    }

    fn success_response() -> Response {
        let mut response = Json(json!({
            "id": "resp_contract",
            "status": "completed",
            "model": "gpt-contract-served",
            "output": [{
                "type": "message",
                "content": [{"type": "output_text", "text": "{\"response\":\"ok\"}"}]
            }],
            "usage": {"input_tokens": 12, "output_tokens": 5}
        }))
        .into_response();
        response.headers_mut().insert(
            "x-request-id",
            HeaderValue::from_static("request_contract_123"),
        );
        response
    }

    fn operation_response(operation_name: Option<&str>) -> Response {
        let output = match operation_name {
            Some("ai_center_agent_turn") => {
                json!({"response": "ok", "proposals": [], "sources": []})
            }
            Some("ai_center_context_selection") => json!({"selected_version_ids": []}),
            Some("ai_center_technical_plan") => json!({
                "title": "Plan",
                "summary": "Résumé",
                "architecture": "Architecture",
                "delivery_slices": [],
                "risks": [],
                "validation": [],
                "coverage": []
            }),
            Some("ai_center_steward_assessment") => json!({"assessments": []}),
            Some("ai_center_coverage_assessment") => json!({"requirements": []}),
            _ => return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        };
        let text = serde_json::to_string(&output).expect("fixture output should serialize");
        let mut response = Json(json!({
            "id": "resp_operation_contract",
            "status": "completed",
            "model": "gpt-contract-served",
            "output": [{
                "type": "message",
                "content": [{"type": "output_text", "text": text}]
            }],
            "usage": {"input_tokens": 7, "output_tokens": 3}
        }))
        .into_response();
        response.headers_mut().insert(
            "x-request-id",
            HeaderValue::from_static("request_operation_contract"),
        );
        response
    }

    fn local_client(server: &FixtureServer, timeout: Duration) -> OpenAiResponsesClient {
        OpenAiResponsesClient::new_for_local_development(
            SecretString::from("contract-fixture-only"),
            &server.endpoint,
            timeout,
            Duration::from_millis(1),
        )
        .expect("loopback fixture endpoint should be accepted")
    }

    fn schema() -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["response"],
            "properties": {"response": {"type": "string"}}
        })
    }

    async fn request(
        client: &OpenAiResponsesClient,
    ) -> Result<super::OpenAiStructuredResponse<FixtureOutput>, AppError> {
        client
            .request_structured(
                "gpt-contract-requested",
                "contract_operation",
                "Return the fixture contract.",
                &json!({"project_id": "fixture"}),
                &schema(),
            )
            .await
    }

    fn assert_provider_error(
        error: &AppError,
        expected_class: ProviderErrorClass,
        expected_attempts: u32,
        expected_status: Option<u16>,
        expected_retryable: bool,
    ) {
        let AppError::Provider(error) = error else {
            panic!("expected a typed provider error, got {error:?}");
        };
        assert_eq!(error.class(), expected_class);
        assert_eq!(error.attempts(), expected_attempts);
        assert_eq!(error.upstream_status(), expected_status);
        assert_eq!(error.is_retryable(), expected_retryable);
    }

    #[tokio::test]
    async fn sends_responses_api_structured_outputs_contract_and_collects_usage() {
        let server = start_fixture(FixtureMode::Success).await;
        let result = request(&local_client(&server, Duration::from_secs(1)))
            .await
            .expect("valid fixture response should deserialize");

        assert_eq!(
            result.output,
            FixtureOutput {
                response: "ok".into()
            }
        );
        assert_eq!(
            result.metadata.served_model.as_deref(),
            Some("gpt-contract-served")
        );
        assert_eq!(
            result.metadata.provider_response_id.as_deref(),
            Some("resp_contract")
        );
        assert_eq!(
            result.metadata.provider_request_id.as_deref(),
            Some("request_contract_123")
        );
        assert_eq!(result.metadata.status.as_deref(), Some("completed"));
        assert_eq!(result.metadata.input_tokens, Some(12));
        assert_eq!(result.metadata.output_tokens, Some(5));
        assert_eq!(result.metadata.attempts, 1);

        let bodies = server
            .bodies
            .lock()
            .expect("fixture body lock should not be poisoned");
        assert_eq!(bodies.len(), 1);
        assert_eq!(bodies[0].get("store"), Some(&Value::Bool(false)));
        assert_eq!(
            bodies[0]
                .pointer("/text/format/type")
                .and_then(Value::as_str),
            Some("json_schema")
        );
        assert_eq!(
            bodies[0]
                .pointer("/text/format/name")
                .and_then(Value::as_str),
            Some("contract_operation")
        );
        assert_eq!(
            bodies[0]
                .pointer("/text/format/strict")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(bodies[0].pointer("/text/format/schema"), Some(&schema()));
    }

    #[tokio::test]
    async fn openai_engine_routes_all_five_operations_through_the_hardened_client() {
        use crate::agent::{
            AgentEngine, AgentInput, ContextSelectionInput, CoverageEvaluationInput, OpenAiEngine,
            StewardInput, TechnicalPlanInput,
        };

        let server = start_fixture(FixtureMode::OperationContracts).await;
        let engine = OpenAiEngine::new_for_local_development(
            SecretString::from("contract-fixture-only"),
            "gpt-contract-requested".into(),
            &server.endpoint,
            Duration::from_secs(1),
            Duration::from_millis(1),
        )
        .expect("loopback engine should initialize");

        let turn = engine
            .respond(AgentInput {
                scope_kind: "product".into(),
                instructions: "Contract fixture".into(),
                user_message: "Capture the intent".into(),
                context: json!({"knowledge": []}),
            })
            .await
            .expect("agent-turn contract should deserialize");
        assert_eq!(turn.output.response, "ok");
        assert_eq!(turn.metadata.provider, "openai");
        assert_eq!(turn.metadata.requested_model, "gpt-contract-requested");
        assert_eq!(
            turn.metadata.served_model.as_deref(),
            Some("gpt-contract-served")
        );
        assert_eq!(turn.metadata.input_tokens, Some(7));
        assert_eq!(turn.metadata.output_tokens, Some(3));
        assert_eq!(turn.metadata.attempts, 1);
        assert_eq!(turn.metadata.estimated_cost, None);

        let selection = engine
            .select_context(ContextSelectionInput {
                objective: "Select context".into(),
                task_kind: "implementation".into(),
                candidates: json!([]),
            })
            .await
            .expect("context-selection contract should deserialize");
        assert!(selection.output.selected_version_ids.is_empty());

        let plan = engine
            .generate_technical_plan(TechnicalPlanInput {
                objective: "Plan work".into(),
                context_pack: json!({"knowledge": []}),
            })
            .await
            .expect("technical-plan contract should deserialize");
        assert_eq!(plan.output.title, "Plan");

        let steward = engine
            .analyze_contradictions(StewardInput {
                candidate_pairs: json!([]),
            })
            .await
            .expect("steward contract should deserialize");
        assert!(steward.output.assessments.is_empty());

        let coverage = engine
            .evaluate_coverage(CoverageEvaluationInput {
                context_pack: json!({"knowledge": []}),
                technical_plan: plan.output,
            })
            .await
            .expect("coverage assessment contract should deserialize");
        assert!(coverage.output.requirements.is_empty());
        assert_eq!(coverage.metadata.provider, "openai");
        assert_eq!(coverage.metadata.input_tokens, Some(7));
        assert_eq!(coverage.metadata.output_tokens, Some(3));

        let bodies = server
            .bodies
            .lock()
            .expect("fixture body lock should not be poisoned");
        let operation_names = bodies
            .iter()
            .filter_map(|body| body.pointer("/text/format/name").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert_eq!(
            operation_names,
            [
                "ai_center_agent_turn",
                "ai_center_context_selection",
                "ai_center_technical_plan",
                "ai_center_steward_assessment",
                "ai_center_coverage_assessment"
            ]
        );
        assert!(bodies.iter().all(|body| {
            body.get("store") == Some(&Value::Bool(false))
                && body.pointer("/text/format/type").and_then(Value::as_str) == Some("json_schema")
                && body.pointer("/text/format/strict").and_then(Value::as_bool) == Some(true)
        }));
    }

    #[tokio::test]
    async fn does_not_retry_terminal_client_statuses() {
        for (status, class) in [
            (StatusCode::UNAUTHORIZED, ProviderErrorClass::Authentication),
            (StatusCode::FORBIDDEN, ProviderErrorClass::Permission),
            (StatusCode::NOT_FOUND, ProviderErrorClass::NotFound),
            (StatusCode::BAD_REQUEST, ProviderErrorClass::Request),
        ] {
            let server = start_fixture(FixtureMode::Status(status)).await;
            let error = request(&local_client(&server, Duration::from_secs(1)))
                .await
                .expect_err("terminal client status should fail");
            assert_provider_error(&error, class, 1, Some(status.as_u16()), false);
            assert_eq!(server.hits.load(Ordering::SeqCst), 1);
        }
    }

    #[tokio::test]
    async fn retries_rate_limits_with_a_hard_attempt_bound() {
        let server = start_fixture(FixtureMode::RateLimitThenSuccess).await;
        let result = request(&local_client(&server, Duration::from_secs(1)))
            .await
            .expect("third rate-limited request should succeed");
        assert_eq!(result.metadata.attempts, 3);
        assert_eq!(server.hits.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn retries_server_errors_with_a_hard_attempt_bound() {
        let server = start_fixture(FixtureMode::Status(StatusCode::SERVICE_UNAVAILABLE)).await;
        let error = request(&local_client(&server, Duration::from_secs(1)))
            .await
            .expect_err("persistent server errors should exhaust retries");
        assert_provider_error(
            &error,
            ProviderErrorClass::Server,
            3,
            Some(StatusCode::SERVICE_UNAVAILABLE.as_u16()),
            true,
        );
        assert_eq!(server.hits.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn classifies_exhausted_rate_limits_as_retryable_after_the_local_bound() {
        let server = start_fixture(FixtureMode::RateLimit).await;
        let error = request(&local_client(&server, Duration::from_secs(1)))
            .await
            .expect_err("persistent rate limits should exhaust retries");
        assert_provider_error(
            &error,
            ProviderErrorClass::RateLimit,
            3,
            Some(StatusCode::TOO_MANY_REQUESTS.as_u16()),
            true,
        );
        assert_eq!(error.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(server.hits.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn insufficient_quota_is_permanent_without_retrying_or_retaining_the_body() {
        let server = start_fixture(FixtureMode::InsufficientQuota).await;
        let error = request(&local_client(&server, Duration::from_secs(1)))
            .await
            .expect_err("exhausted quota should fail immediately");
        assert_provider_error(
            &error,
            ProviderErrorClass::Quota,
            1,
            Some(StatusCode::TOO_MANY_REQUESTS.as_u16()),
            false,
        );
        assert_eq!(error.status_code(), StatusCode::BAD_GATEWAY);
        assert_eq!(error.model_run_error_class(), "provider_quota");
        assert!(
            !error
                .to_string()
                .contains("sensitive fixture billing detail")
        );
        assert_eq!(server.hits.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retries_timeouts_with_a_hard_attempt_bound() {
        let server = start_fixture(FixtureMode::Slow).await;
        let error = request(&local_client(&server, Duration::from_millis(10)))
            .await
            .expect_err("persistent timeout should exhaust retries");
        assert_provider_error(&error, ProviderErrorClass::Timeout, 3, None, true);
        assert_eq!(server.hits.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn retries_connection_failures_with_a_hard_attempt_bound() {
        let server = start_fixture(FixtureMode::Success).await;
        server.task.abort();
        sleep(Duration::from_millis(5)).await;

        let error = request(&local_client(&server, Duration::from_millis(100)))
            .await
            .expect_err("closed loopback endpoint should exhaust connection retries");
        assert_provider_error(&error, ProviderErrorClass::Connection, 3, None, true);
        assert_eq!(server.hits.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn rejects_unreadable_api_json_without_retrying() {
        let server = start_fixture(FixtureMode::InvalidApiJson).await;
        let error = request(&local_client(&server, Duration::from_secs(1)))
            .await
            .expect_err("invalid API JSON should fail");
        assert_provider_error(&error, ProviderErrorClass::ResponseContract, 1, None, false);
        assert!(!error.to_string().contains("not-json"));
        assert_eq!(server.hits.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn rejects_missing_structured_output_without_retrying() {
        let server = start_fixture(FixtureMode::MissingStructuredOutput).await;
        let error = request(&local_client(&server, Duration::from_secs(1)))
            .await
            .expect_err("missing structured output should fail");
        assert_provider_error(&error, ProviderErrorClass::ResponseContract, 1, None, false);
        assert_eq!(server.hits.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn rejects_structured_output_that_does_not_match_the_expected_type() {
        let server = start_fixture(FixtureMode::InvalidStructuredOutput).await;
        let error = request(&local_client(&server, Duration::from_secs(1)))
            .await
            .expect_err("schema-incompatible output should fail");
        assert_provider_error(&error, ProviderErrorClass::ResponseContract, 1, None, false);
        assert!(!error.to_string().contains("unexpected"));
        assert_eq!(server.hits.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn local_endpoint_override_rejects_ssrf_and_ambiguous_urls() {
        for endpoint in [
            "https://example.com/v1/responses",
            "http://localhost.evil.test/v1/responses",
            "http://127.0.0.1:8080/other",
            "http://user:password@127.0.0.1:8080/v1/responses",
            "http://127.0.0.1:8080/v1/responses?next=https://example.com",
            "file:///v1/responses",
        ] {
            let result = OpenAiResponsesClient::new_for_local_development(
                SecretString::from("contract-fixture-only"),
                endpoint,
                Duration::from_secs(1),
                Duration::ZERO,
            );
            assert!(matches!(result, Err(AppError::Invalid(_))), "{endpoint}");
        }
    }

    #[test]
    fn production_constructor_is_pinned_to_the_official_responses_endpoint() {
        let client = OpenAiResponsesClient::new(SecretString::from("contract-fixture-only"))
            .expect("production client should initialize");
        assert_eq!(client.endpoint.as_str(), super::RESPONSES_ENDPOINT);
    }
}
