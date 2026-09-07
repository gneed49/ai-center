//! Official, bounded provider protocols. Credentials never choose an origin.
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use reqwest::{Client, Method, StatusCode, redirect::Policy};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use url::Url;

use crate::{
    agent::AgentRunMetadata,
    error::{AppError, AppResult, ProviderError, ProviderErrorClass},
};

const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
const MAX_ATTEMPTS: u32 = 3;
const MAX_OUTPUT_TOKENS: u32 = 8192;
const MAX_MODEL_PAGES: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderModel {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct StructuredResponse {
    pub output: Value,
    pub metadata: AgentRunMetadata,
}

#[async_trait]
pub trait StructuredTransport: Send + Sync {
    fn provider_name(&self) -> &'static str;
    async fn generate(
        &self,
        model: &str,
        operation_name: &str,
        instructions: &str,
        input: &Value,
        schema: &Value,
    ) -> AppResult<StructuredResponse>;
    async fn list_models(&self) -> AppResult<Vec<ProviderModel>>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Protocol {
    Responses,
    Anthropic,
    ChatJson,
    ChatSchema,
}

#[derive(Clone, Copy)]
struct Preset {
    id: &'static str,
    origin: &'static str,
    generate_path: &'static str,
    models_path: &'static str,
    protocol: Protocol,
}

fn preset(provider: &str) -> AppResult<Preset> {
    let (id, origin, generate_path, models_path, protocol) = match provider {
        "openai" => (
            "openai",
            "https://api.openai.com",
            "/v1/responses",
            "/v1/models",
            Protocol::Responses,
        ),
        "anthropic" => (
            "anthropic",
            "https://api.anthropic.com",
            "/v1/messages",
            "/v1/models",
            Protocol::Anthropic,
        ),
        "kimi" => (
            "kimi",
            "https://api.moonshot.ai",
            "/v1/chat/completions",
            "/v1/models",
            Protocol::ChatJson,
        ),
        "deepseek" => (
            "deepseek",
            "https://api.deepseek.com",
            "/responses",
            "/models",
            Protocol::Responses,
        ),
        "openrouter" => (
            "openrouter",
            "https://openrouter.ai",
            "/api/v1/chat/completions",
            "/api/v1/models/user",
            Protocol::ChatSchema,
        ),
        _ => return Err(AppError::Invalid("unsupported AI provider".into())),
    };
    Ok(Preset {
        id,
        origin,
        generate_path,
        models_path,
        protocol,
    })
}

/// Construct a fixed official endpoint. No endpoint supplied by the user is accepted.
///
/// # Errors
/// Rejects unknown providers or invalid credentials/client configuration.
pub fn api_transport(provider: &str, key: SecretString) -> AppResult<Arc<dyn StructuredTransport>> {
    Ok(Arc::new(ApiTransport::new(provider, key)?))
}

pub struct ApiTransport {
    client: Client,
    key: SecretString,
    preset: Preset,
    origin: Url,
    retry_delay: Duration,
    catalogue_timeout: Duration,
}

impl ApiTransport {
    /// # Errors
    /// Rejects invalid keys and unknown provider identifiers.
    pub fn new(provider: &str, key: SecretString) -> AppResult<Self> {
        let preset = preset(provider)?;
        let origin = Url::parse(preset.origin)
            .map_err(|_| AppError::Internal("invalid provider preset".into()))?;
        Self::build(
            preset,
            key,
            origin,
            Duration::from_secs(60),
            Duration::from_millis(250),
        )
    }

    /// Loopback fixture override is unavailable in optimized release builds.
    ///
    /// # Errors
    /// Rejects any non-loopback origin, path mismatch or URL credentials/query/fragment.
    #[cfg(debug_assertions)]
    pub fn new_for_local_development(
        provider: &str,
        key: SecretString,
        endpoint: &str,
        timeout: Duration,
        retry_delay: Duration,
    ) -> AppResult<Self> {
        let preset = preset(provider)?;
        let mut origin = Url::parse(endpoint)
            .map_err(|_| AppError::Invalid("invalid local provider endpoint".into()))?;
        let loopback = match origin.host() {
            Some(url::Host::Ipv4(address)) => address.is_loopback(),
            Some(url::Host::Ipv6(address)) => address.is_loopback(),
            Some(url::Host::Domain(domain)) => domain == "localhost",
            None => false,
        };
        if !loopback
            || !matches!(origin.scheme(), "http" | "https")
            || origin.path() != preset.generate_path
            || !origin.username().is_empty()
            || origin.password().is_some()
            || origin.query().is_some()
            || origin.fragment().is_some()
        {
            return Err(AppError::Invalid(
                "local provider endpoint must be an exact loopback API URL".into(),
            ));
        }
        origin.set_path("/");
        Self::build(preset, key, origin, timeout, retry_delay)
    }

    fn build(
        preset: Preset,
        key: SecretString,
        origin: Url,
        timeout: Duration,
        retry_delay: Duration,
    ) -> AppResult<Self> {
        if key.expose_secret().is_empty()
            || key.expose_secret().len() > 8192
            || key.expose_secret().chars().any(char::is_control)
        {
            return Err(AppError::Invalid("invalid provider credential".into()));
        }
        let client = Client::builder()
            .connect_timeout(timeout.min(Duration::from_secs(10)))
            .timeout(timeout)
            .redirect(Policy::none())
            .build()
            .map_err(|_| AppError::Internal("failed to build provider client".into()))?;
        Ok(Self {
            client,
            key,
            preset,
            origin,
            retry_delay,
            catalogue_timeout: timeout.min(Duration::from_secs(30)),
        })
    }

    fn error(&self, class: ProviderErrorClass, attempts: u32, status: Option<u16>) -> AppError {
        ProviderError::new(self.preset.id, class, attempts, status).into()
    }

    fn url(&self, path: &str) -> Url {
        let mut url = self.origin.clone();
        url.set_path(path);
        url
    }

    async fn send(
        &self,
        method: Method,
        url: Url,
        body: Option<&Value>,
    ) -> AppResult<(Value, Option<String>, u32)> {
        let body = body
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|_| self.error(ProviderErrorClass::Request, 1, None))?;
        if body
            .as_ref()
            .is_some_and(|body| body.len() > MAX_BODY_BYTES)
        {
            return Err(self.error(ProviderErrorClass::Request, 1, None));
        }
        for attempt in 1..=MAX_ATTEMPTS {
            let mut request = self.client.request(method.clone(), url.clone());
            request = if self.preset.protocol == Protocol::Anthropic {
                request
                    .header("x-api-key", self.key.expose_secret())
                    .header("anthropic-version", "2023-06-01")
            } else {
                request.bearer_auth(self.key.expose_secret())
            };
            if let Some(body) = &body {
                request = request
                    .header("content-type", "application/json")
                    .body(body.clone());
            }
            let response = request.send().await;
            let mut response = match response {
                Ok(response) => response,
                Err(error) => {
                    let class = if error.is_timeout() {
                        ProviderErrorClass::Timeout
                    } else if error.is_connect() {
                        ProviderErrorClass::Connection
                    } else {
                        ProviderErrorClass::Transport
                    };
                    if class.is_retryable() && attempt < MAX_ATTEMPTS {
                        self.wait(attempt).await;
                        continue;
                    }
                    return Err(self.error(class, attempt, None));
                }
            };
            let status = response.status();
            if status.is_server_error() && attempt < MAX_ATTEMPTS {
                self.wait(attempt).await;
                continue;
            }
            if !status.is_success() && status != StatusCode::TOO_MANY_REQUESTS {
                return Err(self.error(classify_status(status), attempt, Some(status.as_u16())));
            }
            let request_id = ["x-request-id", "request-id"]
                .into_iter()
                .find_map(|key| {
                    response
                        .headers()
                        .get(key)
                        .and_then(|value| value.to_str().ok())
                })
                .and_then(safe_identifier);
            let bytes = self.read_response(&mut response, attempt).await?;
            if status == StatusCode::TOO_MANY_REQUESTS {
                let quota = serde_json::from_slice::<Value>(&bytes)
                    .ok()
                    .is_some_and(|value| {
                        ["/error/code", "/error/type"].into_iter().any(|path| {
                            value.pointer(path).and_then(Value::as_str)
                                == Some("insufficient_quota")
                        })
                    });
                let class = if quota {
                    ProviderErrorClass::Quota
                } else {
                    ProviderErrorClass::RateLimit
                };
                if class.is_retryable() && attempt < MAX_ATTEMPTS {
                    self.wait(attempt).await;
                    continue;
                }
                return Err(self.error(class, attempt, Some(status.as_u16())));
            }
            let value = serde_json::from_slice(&bytes)
                .map_err(|_| self.error(ProviderErrorClass::ResponseContract, attempt, None))?;
            return Ok((value, request_id, attempt));
        }
        Err(self.error(ProviderErrorClass::Transport, MAX_ATTEMPTS, None))
    }

    async fn read_response(
        &self,
        response: &mut reqwest::Response,
        attempt: u32,
    ) -> AppResult<Vec<u8>> {
        if response
            .content_length()
            .is_some_and(|len| len > MAX_BODY_BYTES as u64)
        {
            return Err(self.error(ProviderErrorClass::ResponseContract, attempt, None));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| self.error(ProviderErrorClass::ResponseContract, attempt, None))?
        {
            if bytes.len() + chunk.len() > MAX_BODY_BYTES {
                return Err(self.error(ProviderErrorClass::ResponseContract, attempt, None));
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }

    async fn wait(&self, attempt: u32) {
        tokio::time::sleep(self.retry_delay.saturating_mul(attempt)).await;
    }

    async fn fetch_models(&self) -> AppResult<Vec<ProviderModel>> {
        let mut url = self.url(self.preset.models_path);
        if self.preset.protocol == Protocol::Anthropic {
            url.query_pairs_mut().append_pair("limit", "1000");
        }
        let mut models = Vec::new();
        let mut previous_cursor = None;
        for _ in 0..MAX_MODEL_PAGES {
            let (value, _, attempts) = self.send(Method::GET, url.clone(), None).await?;
            let data = value
                .get("data")
                .and_then(Value::as_array)
                .ok_or_else(|| self.error(ProviderErrorClass::ResponseContract, attempts, None))?;
            for model in data {
                let id = model
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(safe_identifier)
                    .filter(|id| id.len() <= 256)
                    .ok_or_else(|| {
                        self.error(ProviderErrorClass::ResponseContract, attempts, None)
                    })?;
                let name = model
                    .get("display_name")
                    .or_else(|| model.get("name"))
                    .and_then(Value::as_str)
                    .filter(|name| name.len() <= 512 && !name.chars().any(char::is_control))
                    .unwrap_or(&id)
                    .to_owned();
                models.push(ProviderModel { id, name });
            }
            if self.preset.protocol != Protocol::Anthropic
                || value.get("has_more") == Some(&json!(false))
            {
                models.sort_by(|left, right| left.id.cmp(&right.id));
                models.dedup_by(|left, right| left.id == right.id);
                return Ok(models);
            }
            if value.get("has_more") != Some(&json!(true)) {
                return Err(self.error(ProviderErrorClass::ResponseContract, attempts, None));
            }
            let cursor = value
                .get("last_id")
                .and_then(Value::as_str)
                .and_then(safe_identifier)
                .filter(|cursor| Some(cursor) != previous_cursor.as_ref())
                .ok_or_else(|| self.error(ProviderErrorClass::ResponseContract, attempts, None))?;
            previous_cursor = Some(cursor.clone());
            url.set_query(None);
            url.query_pairs_mut()
                .append_pair("limit", "1000")
                .append_pair("after_id", &cursor);
        }
        Err(self.error(ProviderErrorClass::ResponseContract, 1, None))
    }

    fn request_body(
        &self,
        model: &str,
        name: &str,
        instructions: &str,
        input: &Value,
        schema: &Value,
    ) -> Value {
        match self.preset.protocol {
            Protocol::Responses => {
                let mut body = json!({"model": model, "instructions": instructions, "input": [{"role": "user", "content": [{"type": "input_text", "text": input.to_string()}]}], "max_output_tokens": MAX_OUTPUT_TOKENS, "stream": false, "text": {"format": {"type": "json_schema", "name": name, "schema": schema}}});
                if self.preset.id == "openai" {
                    body["store"] = json!(false);
                    body["text"]["format"]["strict"] = json!(true);
                }
                body
            }
            Protocol::Anthropic => {
                json!({"model": model, "system": instructions, "messages": [{"role": "user", "content": input.to_string()}], "max_tokens": MAX_OUTPUT_TOKENS, "stream": false, "output_config": {"format": {"type": "json_schema", "schema": anthropic_schema(schema)}}})
            }
            Protocol::ChatJson | Protocol::ChatSchema => {
                let instructions = if self.preset.protocol == Protocol::ChatJson {
                    format!(
                        "{instructions}\nReturn exactly one JSON object matching this JSON Schema: {schema}"
                    )
                } else {
                    instructions.to_owned()
                };
                let mut body = json!({"model": model, "messages": [{"role": "system", "content": instructions}, {"role": "user", "content": input.to_string()}], "max_tokens": MAX_OUTPUT_TOKENS, "stream": false});
                if self.preset.protocol == Protocol::ChatJson {
                    body["response_format"] = json!({"type": "json_object"});
                    body.as_object_mut()
                        .expect("request body is an object")
                        .remove("max_tokens");
                    body["max_completion_tokens"] = json!(MAX_OUTPUT_TOKENS);
                } else {
                    body["response_format"] = json!({"type": "json_schema", "json_schema": {"name": name, "strict": true, "schema": schema}});
                    body["provider"] =
                        json!({"require_parameters": true, "allow_fallbacks": false});
                }
                body
            }
        }
    }
}

#[async_trait]
impl StructuredTransport for ApiTransport {
    fn provider_name(&self) -> &'static str {
        self.preset.id
    }

    async fn generate(
        &self,
        model: &str,
        operation_name: &str,
        instructions: &str,
        input: &Value,
        schema: &Value,
    ) -> AppResult<StructuredResponse> {
        if safe_identifier(model).is_none() || model.len() > 256 {
            return Err(AppError::Invalid("invalid provider model".into()));
        }
        let validator = compile_schema(schema)?;
        let body = self.request_body(model, operation_name, instructions, input, schema);
        let started = Instant::now();
        let (value, request_id, attempts) = self
            .send(
                Method::POST,
                self.url(self.preset.generate_path),
                Some(&body),
            )
            .await?;
        let output_text = output_text(self.preset.protocol, &value)
            .ok_or_else(|| self.error(ProviderErrorClass::ResponseContract, attempts, None))?;
        let output: Value = serde_json::from_str(&output_text)
            .map_err(|_| self.error(ProviderErrorClass::ResponseContract, attempts, None))?;
        if !validator.is_valid(&output) {
            return Err(self.error(ProviderErrorClass::ResponseContract, attempts, None));
        }
        let usage_fields = match self.preset.protocol {
            Protocol::Responses | Protocol::Anthropic => ("input_tokens", "output_tokens"),
            _ => ("prompt_tokens", "completion_tokens"),
        };
        let tokens = |field: &str| {
            value
                .get("usage")
                .and_then(|usage| usage.get(field))
                .and_then(Value::as_i64)
                .filter(|count| *count >= 0)
        };
        Ok(StructuredResponse {
            output,
            metadata: AgentRunMetadata {
                provider: self.preset.id.into(),
                requested_model: model.into(),
                served_model: value
                    .get("model")
                    .and_then(Value::as_str)
                    .and_then(safe_identifier),
                provider_response_id: value
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(safe_identifier),
                provider_request_id: request_id,
                status: Some("completed".into()),
                input_tokens: tokens(usage_fields.0),
                output_tokens: tokens(usage_fields.1),
                estimated_cost: None,
                latency_ms: i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX),
                attempts: i32::try_from(attempts).unwrap_or(i32::MAX),
            },
        })
    }

    async fn list_models(&self) -> AppResult<Vec<ProviderModel>> {
        tokio::time::timeout(self.catalogue_timeout, self.fetch_models())
            .await
            .map_err(|_| self.error(ProviderErrorClass::Timeout, 1, None))?
    }
}

fn safe_identifier(value: &str) -> Option<String> {
    (!value.is_empty()
        && value.len() <= 512
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "-_.:/".contains(ch)))
    .then(|| value.to_owned())
}

fn classify_status(status: StatusCode) -> ProviderErrorClass {
    match status {
        StatusCode::UNAUTHORIZED => ProviderErrorClass::Authentication,
        StatusCode::FORBIDDEN => ProviderErrorClass::Permission,
        StatusCode::NOT_FOUND => ProviderErrorClass::NotFound,
        StatusCode::PAYMENT_REQUIRED => ProviderErrorClass::Quota,
        status if status.is_server_error() => ProviderErrorClass::Server,
        _ => ProviderErrorClass::Request,
    }
}

fn output_text(protocol: Protocol, value: &Value) -> Option<String> {
    if value.get("error").is_some_and(|error| !error.is_null()) {
        return None;
    }
    match protocol {
        Protocol::Responses => {
            if value.get("status")?.as_str()? != "completed"
                || value
                    .get("incomplete_details")
                    .is_some_and(|v| !v.is_null())
            {
                return None;
            }
            let mut text = String::new();
            let mut messages = 0;
            for item in value.get("output")?.as_array()? {
                match item.get("type")?.as_str()? {
                    "reasoning" => {}
                    "message" => {
                        messages += 1;
                        if item
                            .get("status")
                            .is_some_and(|v| v.as_str() != Some("completed"))
                        {
                            return None;
                        }
                        for content in item.get("content")?.as_array()? {
                            if content.get("type")?.as_str()? != "output_text" {
                                return None;
                            }
                            text.push_str(content.get("text")?.as_str()?);
                        }
                    }
                    _ => return None,
                }
            }
            (messages == 1 && !text.is_empty()).then_some(text)
        }
        Protocol::Anthropic => {
            if value.get("stop_reason")?.as_str()? != "end_turn" {
                return None;
            }
            let mut text = String::new();
            for content in value.get("content")?.as_array()? {
                if content.get("type")?.as_str()? != "text" {
                    return None;
                }
                text.push_str(content.get("text")?.as_str()?);
            }
            (!text.is_empty()).then_some(text)
        }
        Protocol::ChatJson | Protocol::ChatSchema => {
            let choices = value.get("choices")?.as_array()?;
            if choices.len() != 1
                || choices[0].get("finish_reason")?.as_str()? != "stop"
                || choices[0].get("error").is_some_and(|v| !v.is_null())
            {
                return None;
            }
            let message = choices[0].get("message")?;
            if message.get("refusal").is_some_and(|v| !v.is_null())
                || message.get("tool_calls").is_some_and(|v| {
                    !v.is_null() && v.as_array().is_none_or(|calls| !calls.is_empty())
                })
            {
                return None;
            }
            message
                .get("content")?
                .as_str()
                .filter(|text| !text.is_empty())
                .map(str::to_owned)
        }
    }
}

/// Anthropic documents a smaller schema grammar than the server contract.
/// Move unsupported constraints to descriptions, then validate the answer against
/// the original schema. This mirrors the official SDK transformation.
fn anthropic_schema(schema: &Value) -> Value {
    let mut schema = schema.clone();
    let Some(node) = schema.as_object_mut() else {
        return schema;
    };
    let mut constraints = Vec::new();
    for keyword in [
        "minimum",
        "maximum",
        "exclusiveMinimum",
        "exclusiveMaximum",
        "multipleOf",
        "minLength",
        "maxLength",
        "maxItems",
        "uniqueItems",
    ] {
        if let Some(value) = node.remove(keyword) {
            constraints.push(format!("{keyword}: {value}"));
        }
    }
    if node
        .get("minItems")
        .and_then(Value::as_u64)
        .is_some_and(|count| count > 1)
    {
        if let Some(value) = node.remove("minItems") {
            constraints.push(format!("minItems: {value}"));
        }
        node.insert("minItems".into(), json!(1));
    }
    if !constraints.is_empty() {
        let prior = node
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("");
        node.insert(
            "description".into(),
            json!(format!("{prior} Required constraints: {}.", constraints.join("; ")).trim()),
        );
    }
    for keyword in ["properties", "$defs", "definitions"] {
        if let Some(children) = node.get_mut(keyword).and_then(Value::as_object_mut) {
            for child in children.values_mut() {
                *child = anthropic_schema(child);
            }
        }
    }
    for keyword in ["items", "additionalProperties"] {
        if let Some(child) = node.get_mut(keyword) {
            *child = anthropic_schema(child);
        }
    }
    for keyword in ["anyOf", "allOf", "oneOf", "prefixItems"] {
        if let Some(children) = node.get_mut(keyword).and_then(Value::as_array_mut) {
            for child in children {
                *child = anthropic_schema(child);
            }
        }
    }
    schema
}

struct DenyExternalSchemas;
impl jsonschema::Retrieve for DenyExternalSchemas {
    fn retrieve(
        &self,
        _uri: &jsonschema::Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Err("external schema retrieval is disabled".into())
    }
}

fn compile_schema(schema: &Value) -> AppResult<jsonschema::Validator> {
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .should_validate_formats(true)
        .with_retriever(DenyExternalSchemas)
        .build(schema)
        .map_err(|_| AppError::Internal("invalid built-in structured output schema".into()))
}

/// Apply the same server contract to API and local subscription transports.
///
/// # Errors
/// Rejects invalid schemas or any output violating the schema, including formats.
pub fn validate_output(provider: &'static str, schema: &Value, output: &Value) -> AppResult<()> {
    if !compile_schema(schema)?.is_valid(output) {
        return Err(
            ProviderError::new(provider, ProviderErrorClass::ResponseContract, 1, None).into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests;
