//! Bounded official API adapters. Remote error text and credentials never escape.
#![allow(clippy::missing_errors_doc)]
use reqwest::{Client as HttpClient, Method, header};
use secrecy::{ExposeSecret, SecretString};
use serde_json::{Value, json};
use std::time::Duration;
use uuid::Uuid;

use super::models::Receipt;
use crate::error::{AppError, AppResult};

const MAX_RESPONSE: usize = 262_144;
const ISSUE_FIELDS: &str = "id url title description updatedAt team { id }";

#[derive(Debug)]
pub struct RemoteError {
    pub code: &'static str,
    pub ambiguous: bool,
}
impl RemoteError {
    fn contract() -> Self {
        Self {
            code: "remote_response_invalid",
            ambiguous: true,
        }
    }
}
pub struct ToolClient {
    client: HttpClient,
    loopback: Option<String>,
}
impl ToolClient {
    pub fn official() -> AppResult<Self> {
        Self::build(None, Duration::from_secs(30))
    }
    fn build(loopback: Option<String>, timeout: Duration) -> AppResult<Self> {
        let client = HttpClient::builder()
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .user_agent("AI-Center/1.0")
            .build()
            .map_err(|_| AppError::Internal("Work tool HTTP client unavailable".into()))?;
        Ok(Self { client, loopback })
    }
    /// Only compiled for local contract tests; never accepts a remote host.
    #[cfg(debug_assertions)]
    pub fn loopback(base: &str, timeout: Duration) -> AppResult<Self> {
        let url =
            url::Url::parse(base).map_err(|_| AppError::Invalid("Invalid loopback URL".into()))?;
        if url.scheme() != "http"
            || url.host_str() != Some("127.0.0.1")
            || url.port().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/"
        {
            return Err(AppError::Invalid(
                "Only an explicit loopback test server is permitted".into(),
            ));
        }
        Self::build(Some(base.trim_end_matches('/').into()), timeout)
    }
    fn endpoint(&self, provider: &str, path: &str) -> String {
        if let Some(base) = &self.loopback {
            return format!("{base}/{provider}{path}");
        }
        let host = match provider {
            "notion" => "https://api.notion.com",
            "linear" => "https://api.linear.app",
            _ => "https://api.github.com",
        };
        format!("{host}{path}")
    }
    pub(super) async fn request(
        &self,
        provider: &str,
        secret: &SecretString,
        method: Method,
        path: &str,
        body: Option<&Value>,
        creating: bool,
    ) -> Result<Value, RemoteError> {
        if !matches!(provider, "notion" | "linear" | "github") {
            return Err(RemoteError::contract());
        }
        let auth = if provider == "linear" {
            secret.expose_secret().to_owned()
        } else {
            format!("Bearer {}", secret.expose_secret())
        };
        let mut header = header::HeaderValue::from_str(&auth).map_err(|_| RemoteError {
            code: "invalid_credential",
            ambiguous: false,
        })?;
        header.set_sensitive(true);
        let mut request = self
            .client
            .request(method, self.endpoint(provider, path))
            .header(header::AUTHORIZATION, header)
            .header(header::ACCEPT, "application/json");
        if provider == "notion" {
            request = request.header("Notion-Version", "2026-03-11");
        }
        if provider == "github" {
            request = request.header("X-GitHub-Api-Version", "2022-11-28");
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        let mut response = request.send().await.map_err(|error| RemoteError {
            code: if error.is_timeout() {
                "remote_timeout"
            } else {
                "remote_transport"
            },
            ambiguous: creating,
        })?;
        let status = response.status();
        if !status.is_success() {
            return Err(RemoteError {
                code: match status.as_u16() {
                    401 => "remote_authentication",
                    403 => "remote_permission",
                    404 => "remote_not_found",
                    429 => "remote_rate_limit",
                    300..=399 => "remote_redirect_blocked",
                    400..=499 => "remote_request_rejected",
                    _ => "remote_unavailable",
                },
                ambiguous: creating
                    && (status.is_server_error()
                        || status.is_redirection()
                        || status.as_u16() == 408),
            });
        }
        if response
            .content_length()
            .is_some_and(|len| len > MAX_RESPONSE as u64)
        {
            return Err(RemoteError::contract());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| RemoteError {
            code: "remote_response_interrupted",
            ambiguous: creating,
        })? {
            if bytes.len() + chunk.len() > MAX_RESPONSE {
                return Err(RemoteError::contract());
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&bytes).map_err(|_| RemoteError::contract())?;
        if provider == "linear"
            && value
                .get("errors")
                .is_some_and(|errors| errors.as_array().is_none_or(|errors| !errors.is_empty()))
        {
            return Err(RemoteError {
                code: "remote_graphql_errors",
                ambiguous: creating,
            });
        }
        Ok(value)
    }
    pub async fn test(&self, provider: &str, secret: &SecretString) -> Result<(), RemoteError> {
        let value = match provider {
            "notion" => {
                self.request(provider, secret, Method::GET, "/v1/users/me", None, false)
                    .await?
            }
            "linear" => {
                self.request(
                    provider,
                    secret,
                    Method::POST,
                    "/graphql",
                    Some(&json!({"query":"query { viewer { id } }"})),
                    false,
                )
                .await?
            }
            "github" => {
                self.request(provider, secret, Method::GET, "/user", None, false)
                    .await?
            }
            _ => return Err(RemoteError::contract()),
        };
        if (provider == "linear"
            && value
                .pointer("/data/viewer/id")
                .and_then(Value::as_str)
                .is_some())
            || (provider == "notion" && value["id"].as_str().is_some())
            || (provider == "github" && value["id"].as_i64().is_some_and(|id| id > 0))
        {
            Ok(())
        } else {
            Err(RemoteError::contract())
        }
    }
    pub async fn create(
        &self,
        provider: &str,
        secret: &SecretString,
        target: &str,
        title: &str,
        body: &str,
    ) -> Result<Receipt, RemoteError> {
        let value=match provider{
            "notion"=>self.request(provider,secret,Method::POST,"/v1/pages",Some(&json!({"parent":{"page_id":target},"properties":{"title":{"type":"title","title":[{"type":"text","text":{"content":title}}]}},"markdown":body})),true).await?,
            "linear"=>self.request(provider,secret,Method::POST,"/graphql",Some(&json!({"query":format!("mutation Publish($input: IssueCreateInput!) {{ issueCreate(input: $input) {{ success issue {{ {ISSUE_FIELDS} }} }} }}"),"variables":{"input":{"teamId":target,"title":title,"description":body}}})),true).await?,
            "github"=>self.request(provider,secret,Method::POST,&format!("/repos/{target}/issues"),Some(&json!({"title":title,"body":body})),true).await?,
            _=>return Err(RemoteError::contract()),
        };
        match provider {
            "notion" => {
                let mut receipt = notion_page(&value, target)?;
                // Create confirms the page identity, not a subsequent canonical read.
                receipt.body_markdown = body.into();
                receipt.complete = false;
                Ok(receipt)
            }
            "linear" => {
                if value.pointer("/data/issueCreate/success") != Some(&Value::Bool(true)) {
                    return Err(RemoteError::contract());
                }
                linear_issue(&value["data"]["issueCreate"]["issue"], target)
            }
            _ => github_issue(&value, target),
        }
    }
    pub async fn read(
        &self,
        provider: &str,
        secret: &SecretString,
        target: &str,
        id: &str,
    ) -> Result<Receipt, RemoteError> {
        let id = normalize_external_id(provider, id).ok_or_else(RemoteError::contract)?;
        match provider {
            "notion" => {
                let page = self
                    .request(
                        provider,
                        secret,
                        Method::GET,
                        &format!("/v1/pages/{id}"),
                        None,
                        false,
                    )
                    .await?;
                let mut receipt = notion_page(&page, target)?;
                if receipt.external_id != id {
                    return Err(RemoteError::contract());
                }
                let markdown = self
                    .request(
                        provider,
                        secret,
                        Method::GET,
                        &format!("/v1/pages/{id}/markdown"),
                        None,
                        false,
                    )
                    .await?;
                receipt.body_markdown = field(&markdown, "markdown")?.into();
                receipt.complete = markdown["truncated"] == false
                    && markdown["unknown_block_ids"]
                        .as_array()
                        .is_some_and(Vec::is_empty);
                Ok(receipt)
            }
            "linear" => {
                let value=self.request(provider,secret,Method::POST,"/graphql",Some(&json!({"query":format!("query Read($id: String!) {{ issue(id: $id) {{ {ISSUE_FIELDS} }} }}"),"variables":{"id":id}})),false).await?;
                let receipt = linear_issue(&value["data"]["issue"], target)?;
                if receipt.external_id != id {
                    return Err(RemoteError::contract());
                }
                Ok(receipt)
            }
            "github" => {
                let value = self
                    .request(
                        provider,
                        secret,
                        Method::GET,
                        &format!("/repos/{target}/issues/{id}"),
                        None,
                        false,
                    )
                    .await?;
                let receipt = github_issue(&value, target)?;
                if receipt.external_id != id {
                    return Err(RemoteError::contract());
                }
                Ok(receipt)
            }
            _ => Err(RemoteError::contract()),
        }
    }
}
fn field<'a>(value: &'a Value, key: &str) -> Result<&'a str, RemoteError> {
    value[key].as_str().ok_or_else(RemoteError::contract)
}
fn uuid_field(value: &Value, key: &str) -> Result<String, RemoteError> {
    Uuid::parse_str(field(value, key)?)
        .ok()
        .filter(|id| !id.is_nil())
        .map(|id| id.to_string())
        .ok_or_else(RemoteError::contract)
}
fn notion_page(value: &Value, target: &str) -> Result<Receipt, RemoteError> {
    let id = uuid_field(value, "id")?;
    if value["object"] != "page"
        || value["archived"] == true
        || value["in_trash"] == true
        || uuid_field(&value["parent"], "page_id")? != target
    {
        return Err(RemoteError::contract());
    }
    let title = value
        .pointer("/properties/title/title")
        .and_then(Value::as_array)
        .ok_or_else(RemoteError::contract)?
        .iter()
        .map(|part| {
            part["plain_text"]
                .as_str()
                .or_else(|| part.pointer("/text/content").and_then(Value::as_str))
                .unwrap_or("")
        })
        .collect();
    Ok(Receipt {
        external_url: format!("https://www.notion.so/{}", id.replace('-', "")),
        external_id: id,
        target_id: target.into(),
        title,
        body_markdown: String::new(),
        remote_updated_at: value["last_edited_time"].as_str().map(str::to_owned),
        complete: false,
    })
}
fn linear_issue(value: &Value, target: &str) -> Result<Receipt, RemoteError> {
    let id = uuid_field(value, "id")?;
    let url = field(value, "url")?;
    let parsed = url::Url::parse(url).map_err(|_| RemoteError::contract())?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("linear.app")
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.port().is_some()
        || uuid_field(&value["team"], "id")? != target
    {
        return Err(RemoteError::contract());
    }
    Ok(Receipt {
        external_id: id,
        external_url: url.into(),
        target_id: target.into(),
        title: field(value, "title")?.into(),
        body_markdown: field(value, "description")?.into(),
        remote_updated_at: value["updatedAt"].as_str().map(str::to_owned),
        complete: true,
    })
}
fn github_issue(value: &Value, target: &str) -> Result<Receipt, RemoteError> {
    let number = value["number"]
        .as_u64()
        .filter(|n| *n > 0)
        .ok_or_else(RemoteError::contract)?;
    let url = format!("https://github.com/{target}/issues/{number}");
    if value.get("pull_request").is_some() || !field(value, "html_url")?.eq_ignore_ascii_case(&url)
    {
        return Err(RemoteError::contract());
    }
    Ok(Receipt {
        external_id: number.to_string(),
        external_url: url,
        target_id: target.into(),
        title: field(value, "title")?.into(),
        body_markdown: field(value, "body")?.into(),
        remote_updated_at: value["updated_at"].as_str().map(str::to_owned),
        complete: true,
    })
}
#[must_use]
pub fn normalize_external_id(provider: &str, value: &str) -> Option<String> {
    if value.len() > 256 || value.trim() != value {
        return None;
    }
    match provider {
        "notion" | "linear" => Uuid::parse_str(value)
            .ok()
            .filter(|id| !id.is_nil())
            .map(|id| id.to_string()),
        "github" => value
            .parse::<u64>()
            .ok()
            .filter(|id| *id > 0)
            .map(|id| id.to_string()),
        _ => None,
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
