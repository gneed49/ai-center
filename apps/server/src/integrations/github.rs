use std::{sync::Arc, time::Duration};

use chrono::{DateTime, TimeDelta, Utc};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use reqwest::{Client, StatusCode, header};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::{sync::Mutex, time::sleep};
#[cfg(test)]
use url::Host;
use url::Url;

use crate::error::{AppError, AppResult};

const GITHUB_API: &str = "https://api.github.com";
const GITHUB_API_VERSION: &str = "2026-03-10";
const GITHUB_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const GITHUB_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone)]
struct GitHubApiEndpoint {
    base: Url,
}

impl GitHubApiEndpoint {
    fn production() -> AppResult<Self> {
        let base = Url::parse(GITHUB_API)
            .map_err(|_| AppError::Connector("GitHub API endpoint is invalid".into()))?;
        if base.scheme() != "https"
            || base.host_str() != Some("api.github.com")
            || base.port().is_some()
            || !base.username().is_empty()
            || base.password().is_some()
            || base.path() != "/"
            || base.query().is_some()
            || base.fragment().is_some()
        {
            return Err(AppError::Connector(
                "GitHub production endpoint is not canonical".into(),
            ));
        }
        Ok(Self { base })
    }

    #[cfg(test)]
    fn loopback_for_test(value: &str) -> AppResult<Self> {
        let base = Url::parse(value)
            .map_err(|_| AppError::Connector("invalid GitHub test endpoint".into()))?;
        let loopback = match base.host() {
            Some(Host::Ipv4(address)) => address.is_loopback(),
            Some(Host::Ipv6(address)) => address.is_loopback(),
            Some(Host::Domain(_)) | None => false,
        };
        if !matches!(base.scheme(), "http" | "https")
            || !loopback
            || !base.username().is_empty()
            || base.password().is_some()
            || base.path() != "/"
            || base.query().is_some()
            || base.fragment().is_some()
        {
            return Err(AppError::Connector(
                "GitHub test endpoint must be an explicit loopback origin".into(),
            ));
        }
        Ok(Self { base })
    }

    fn request_url(&self, path: &str) -> AppResult<Url> {
        if !path.starts_with('/') || path.starts_with("//") {
            return Err(AppError::Connector("invalid GitHub API path".into()));
        }
        let url = self
            .base
            .join(path)
            .map_err(|_| AppError::Connector("invalid GitHub API path".into()))?;
        if url.origin() != self.base.origin() {
            return Err(AppError::Connector(
                "GitHub API path changed the configured origin".into(),
            ));
        }
        Ok(url)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubPullRequestIdentity {
    pub owner: String,
    pub repository: String,
    pub number: u64,
}

impl GitHubPullRequestIdentity {
    /// Parses a canonical GitHub pull-request URL without following redirects.
    ///
    /// # Errors
    /// Returns an error for another host, credentials, query parameters, or a
    /// path that is not exactly an owner/repository pull request.
    pub fn parse(value: &str) -> AppResult<Self> {
        let url = Url::parse(value)
            .map_err(|_| AppError::Invalid("invalid GitHub pull request URL".into()))?;
        if !value.starts_with("https://github.com/")
            || url.scheme() != "https"
            || url.host_str() != Some("github.com")
            || url.port().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(AppError::Invalid(
                "only canonical https://github.com pull request URLs are accepted".into(),
            ));
        }
        let segments = url
            .path_segments()
            .map(Iterator::collect::<Vec<_>>)
            .unwrap_or_default();
        if segments.len() != 4 || segments[2] != "pull" {
            return Err(AppError::Invalid(
                "GitHub URL must identify a pull request".into(),
            ));
        }
        let owner = validate_slug(segments[0], "owner")?;
        let repository = validate_slug(segments[1], "repository")?;
        let number = segments[3]
            .parse::<u64>()
            .ok()
            .filter(|number| *number > 0)
            .ok_or_else(|| AppError::Invalid("invalid GitHub pull request number".into()))?;
        Ok(Self {
            owner,
            repository,
            number,
        })
    }

    #[must_use]
    pub fn canonical_url(&self) -> String {
        format!(
            "https://github.com/{}/{}/pull/{}",
            self.owner, self.repository, self.number
        )
    }

    #[must_use]
    pub fn external_id(&self) -> String {
        format!("{}/{}#{}", self.owner, self.repository, self.number)
    }
}

fn validate_slug(value: &str, kind: &str) -> AppResult<String> {
    let valid = !value.is_empty()
        && value.len() <= 100
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        });
    if !valid || value == "." || value == ".." {
        return Err(AppError::Invalid(format!("invalid GitHub {kind}")));
    }
    Ok(value.to_owned())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubPullRequestObservation {
    pub identity: GitHubPullRequestIdentity,
    pub canonical_url: String,
    pub title: String,
    pub state: String,
    pub draft: bool,
    pub merged: bool,
    pub base_sha: String,
    pub head_sha: String,
    pub commits: Vec<String>,
    pub changed_paths: Vec<String>,
    pub checks: Vec<GitHubCheckObservation>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub observed_at: DateTime<Utc>,
}

impl GitHubPullRequestObservation {
    fn has_same_provider_state(&self, previous: &Self) -> bool {
        let mut current = self.clone();
        let mut previous = previous.clone();
        current.observed_at = previous.observed_at;
        normalize_projection_collections(&mut current);
        normalize_projection_collections(&mut previous);
        current == previous
    }
}

fn normalize_projection_collections(observation: &mut GitHubPullRequestObservation) {
    observation.changed_paths.sort();
    observation.changed_paths.dedup();
    observation.checks.sort_by(|left, right| {
        (
            left.name.as_str(),
            left.status.as_str(),
            left.conclusion.as_deref(),
            left.details_url.as_deref(),
        )
            .cmp(&(
                right.name.as_str(),
                right.status.as_str(),
                right.conclusion.as_deref(),
                right.details_url.as_deref(),
            ))
    });
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubCheckObservation {
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub details_url: Option<String>,
}

pub enum GitHubObserveResult {
    NotModified,
    Observed {
        observation: Box<GitHubPullRequestObservation>,
        etag: Option<String>,
    },
}

/// Conditional cursor for a pull-request refresh.
///
/// The provider validator is inseparable from the last complete projection so
/// a PR-level `304` can be reconstructed while commits, files, and checks are
/// still fetched and compared.
#[derive(Debug, Clone, Copy)]
pub struct GitHubPullRequestCursor<'a> {
    pub etag: &'a str,
    pub observation: &'a GitHubPullRequestObservation,
}

#[derive(Clone)]
pub struct GitHubAppCredentials {
    pub app_id: String,
    pub installation_id: u64,
    pub private_key_pem: SecretString,
}

struct CachedToken {
    token: SecretString,
    expires_at: DateTime<Utc>,
}

pub struct GitHubAppTokenProvider {
    client: Client,
    api_endpoint: GitHubApiEndpoint,
    credentials: GitHubAppCredentials,
    cached: Mutex<Option<CachedToken>>,
}

impl GitHubAppTokenProvider {
    /// Builds a GitHub App token provider with the connector's hardened client.
    ///
    /// # Errors
    ///
    /// Returns a connector configuration error if the client cannot be built.
    pub fn new(credentials: GitHubAppCredentials) -> AppResult<Self> {
        let api_endpoint = GitHubApiEndpoint::production()?;
        Ok(Self {
            client: github_http_client(GITHUB_CONNECT_TIMEOUT, GITHUB_REQUEST_TIMEOUT)?,
            api_endpoint,
            credentials,
            cached: Mutex::new(None),
        })
    }

    #[allow(clippy::items_after_statements)]
    async fn installation_token(&self) -> AppResult<SecretString> {
        let mut cached = self.cached.lock().await;
        if let Some(token) = cached.as_ref()
            && token.expires_at > Utc::now() + TimeDelta::minutes(5)
        {
            return Ok(token.token.clone());
        }

        #[derive(Serialize)]
        struct AppClaims<'a> {
            iat: i64,
            exp: i64,
            iss: &'a str,
        }
        #[derive(Deserialize)]
        struct InstallationTokenResponse {
            token: String,
            expires_at: DateTime<Utc>,
        }

        let now = Utc::now();
        let key =
            EncodingKey::from_rsa_pem(self.credentials.private_key_pem.expose_secret().as_bytes())
                .map_err(|_| AppError::Connector("GitHub App private key is invalid".into()))?;
        let jwt = encode(
            &Header::new(Algorithm::RS256),
            &AppClaims {
                iat: (now - TimeDelta::seconds(30)).timestamp(),
                exp: (now + TimeDelta::minutes(9)).timestamp(),
                iss: &self.credentials.app_id,
            },
            &key,
        )
        .map_err(|_| AppError::Connector("GitHub App authentication failed".into()))?;

        let url = self.api_endpoint.request_url(&format!(
            "/app/installations/{}/access_tokens",
            self.credentials.installation_id
        ))?;
        let response = self
            .client
            .post(url)
            .bearer_auth(jwt)
            .json(&json!({
                "permissions": {
                    "contents": "read",
                    "pull_requests": "read",
                    "checks": "read",
                    "statuses": "read"
                }
            }))
            .send()
            .await
            .map_err(|_| AppError::Connector("GitHub App token request failed".into()))?;
        if !response.status().is_success() {
            return Err(AppError::Connector(format!(
                "GitHub App token request returned status {}",
                response.status().as_u16()
            )));
        }
        let issued: InstallationTokenResponse = response
            .json()
            .await
            .map_err(|_| AppError::Connector("GitHub returned an invalid token response".into()))?;
        let token = SecretString::from(issued.token);
        *cached = Some(CachedToken {
            token: token.clone(),
            expires_at: issued.expires_at,
        });
        Ok(token)
    }
}

pub struct GitHubClient {
    client: Client,
    api_endpoint: GitHubApiEndpoint,
    token_source: GitHubTokenSource,
}

enum GitHubTokenSource {
    App(Arc<GitHubAppTokenProvider>),
    #[cfg(test)]
    ContractTest(SecretString),
}

impl GitHubTokenSource {
    async fn token(&self) -> AppResult<SecretString> {
        match self {
            Self::App(provider) => provider.installation_token().await,
            #[cfg(test)]
            Self::ContractTest(token) => Ok(token.clone()),
        }
    }
}

impl GitHubClient {
    /// Builds the read-only GitHub observation client.
    ///
    /// # Errors
    ///
    /// Returns a connector configuration error if redirects and timeouts
    /// cannot be enforced.
    pub fn new(token_provider: Arc<GitHubAppTokenProvider>) -> AppResult<Self> {
        Ok(Self {
            client: github_http_client(GITHUB_CONNECT_TIMEOUT, GITHUB_REQUEST_TIMEOUT)?,
            api_endpoint: GitHubApiEndpoint::production()?,
            token_source: GitHubTokenSource::App(token_provider),
        })
    }

    #[cfg(test)]
    fn for_contract_test(endpoint: &str, request_timeout: Duration) -> AppResult<Self> {
        Ok(Self {
            client: github_http_client(request_timeout, request_timeout)?,
            api_endpoint: GitHubApiEndpoint::loopback_for_test(endpoint)?,
            token_source: GitHubTokenSource::ContractTest(SecretString::from(
                "github-contract-test-token",
            )),
        })
    }

    /// Reads an allowlisted pull-request projection from the GitHub API.
    ///
    /// # Errors
    ///
    /// Returns a sanitized connector error for authentication, transport,
    /// rate-limit exhaustion, invalid JSON, or missing mandatory fields.
    pub async fn observe_pull_request(
        &self,
        identity: GitHubPullRequestIdentity,
        cursor: Option<GitHubPullRequestCursor<'_>>,
    ) -> AppResult<GitHubObserveResult> {
        let token = self.token_source.token().await?;
        let pull_path = format!(
            "/repos/{}/{}/pulls/{}",
            identity.owner, identity.repository, identity.number
        );
        let pull_result = self
            .get_value(&pull_path, &token, cursor.map(|cursor| cursor.etag))
            .await?;
        let (pull_fields, response_etag, previous) = match pull_result {
            GitHubValue::NotModified => {
                let cursor = cursor.ok_or_else(|| {
                    AppError::Connector(
                        "GitHub returned not-modified without a prior projection".into(),
                    )
                })?;
                (
                    GitHubPullFields::from_previous(cursor.observation, &identity)?,
                    Some(cursor.etag.to_owned()),
                    Some(cursor.observation),
                )
            }
            GitHubValue::Value { value, etag } => {
                (GitHubPullFields::from_response(&value)?, etag, None)
            }
        };
        let head_sha = pull_fields.head_sha.clone();
        let commits_path = format!("{pull_path}/commits?per_page=100");
        let files_path = format!("{pull_path}/files?per_page=100");
        let checks_path = format!(
            "/repos/{}/{}/commits/{head_sha}/check-runs?per_page=100",
            identity.owner, identity.repository
        );
        let (commits, files, checks) = tokio::try_join!(
            self.get_value(&commits_path, &token, None),
            self.get_value(&files_path, &token, None),
            self.get_value(&checks_path, &token, None),
        )?;

        let commits = value_only(commits)?;
        let files = value_only(files)?;
        let checks = value_only(checks)?;
        let observation = GitHubPullRequestObservation {
            identity: identity.clone(),
            canonical_url: identity.canonical_url(),
            title: pull_fields.title,
            state: pull_fields.state,
            draft: pull_fields.draft,
            merged: pull_fields.merged,
            base_sha: pull_fields.base_sha,
            head_sha,
            commits: commit_shas(&commits),
            changed_paths: changed_paths(&files),
            checks: check_observations(&checks),
            created_at: pull_fields.created_at,
            updated_at: pull_fields.updated_at,
            observed_at: Utc::now(),
        };
        if previous.is_some_and(|previous| observation.has_same_provider_state(previous)) {
            return Ok(GitHubObserveResult::NotModified);
        }
        Ok(GitHubObserveResult::Observed {
            observation: Box::new(observation),
            etag: response_etag,
        })
    }

    async fn get_value(
        &self,
        path: &str,
        token: &SecretString,
        etag: Option<&str>,
    ) -> AppResult<GitHubValue> {
        for attempt in 1..=2 {
            let mut request = self
                .client
                .get(self.api_endpoint.request_url(path)?)
                .bearer_auth(token.expose_secret());
            if let Some(etag) = etag {
                request = request.header(header::IF_NONE_MATCH, etag);
            }
            let response = request
                .send()
                .await
                .map_err(|_| AppError::Connector("GitHub request failed".into()))?;
            if response.status() == StatusCode::NOT_MODIFIED {
                return Ok(GitHubValue::NotModified);
            }
            if attempt < 2
                && (response.status() == StatusCode::TOO_MANY_REQUESTS
                    || response.status().is_server_error())
            {
                let seconds = response
                    .headers()
                    .get(header::RETRY_AFTER)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(1)
                    .min(5);
                sleep(Duration::from_secs(seconds)).await;
                continue;
            }
            if !response.status().is_success() {
                return Err(AppError::Connector(format!(
                    "GitHub request returned status {}",
                    response.status().as_u16()
                )));
            }
            if has_next_page(response.headers().get(header::LINK)) {
                return Err(AppError::Connector(
                    "GitHub projection exceeds the alpha pagination limit".into(),
                ));
            }
            let response_etag = response
                .headers()
                .get(header::ETAG)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            let value = response
                .json()
                .await
                .map_err(|_| AppError::Connector("GitHub returned invalid JSON".into()))?;
            return Ok(GitHubValue::Value {
                value,
                etag: response_etag,
            });
        }
        Err(AppError::Connector(
            "GitHub request exhausted retries".into(),
        ))
    }
}

fn commit_shas(commits: &Value) -> Vec<String> {
    commits
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("sha").and_then(Value::as_str).map(str::to_owned))
        .collect()
}

fn changed_paths(files: &Value) -> Vec<String> {
    files
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| {
            item.get("filename")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect()
}

fn check_observations(checks: &Value) -> Vec<GitHubCheckObservation> {
    checks
        .get("check_runs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            Some(GitHubCheckObservation {
                name: item.get("name")?.as_str()?.to_owned(),
                status: item.get("status")?.as_str()?.to_owned(),
                conclusion: item
                    .get("conclusion")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                details_url: item
                    .get("details_url")
                    .and_then(Value::as_str)
                    .filter(|url| url.starts_with("https://github.com/"))
                    .map(str::to_owned),
            })
        })
        .collect()
}

struct GitHubPullFields {
    title: String,
    state: String,
    draft: bool,
    merged: bool,
    base_sha: String,
    head_sha: String,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

impl GitHubPullFields {
    fn from_response(pull: &Value) -> AppResult<Self> {
        Ok(Self {
            title: required_string(pull, "/title", "title")?,
            state: required_string(pull, "/state", "state")?,
            draft: pull.get("draft").and_then(Value::as_bool).unwrap_or(false),
            merged: pull.get("merged").and_then(Value::as_bool).unwrap_or(false),
            base_sha: required_git_oid(pull, "/base/sha", "base SHA")?,
            head_sha: required_git_oid(pull, "/head/sha", "head SHA")?,
            created_at: optional_datetime(pull, "/created_at"),
            updated_at: optional_datetime(pull, "/updated_at"),
        })
    }

    fn from_previous(
        previous: &GitHubPullRequestObservation,
        identity: &GitHubPullRequestIdentity,
    ) -> AppResult<Self> {
        if &previous.identity != identity
            || previous.canonical_url != identity.canonical_url()
            || previous.title.trim().is_empty()
            || !matches!(previous.state.as_str(), "open" | "closed")
            || !valid_git_oid(&previous.base_sha)
            || !valid_git_oid(&previous.head_sha)
        {
            return Err(AppError::Connector(
                "stored GitHub projection cannot reconstruct a not-modified response".into(),
            ));
        }
        Ok(Self {
            title: previous.title.clone(),
            state: previous.state.clone(),
            draft: previous.draft,
            merged: previous.merged,
            base_sha: previous.base_sha.clone(),
            head_sha: previous.head_sha.clone(),
            created_at: previous.created_at,
            updated_at: previous.updated_at,
        })
    }
}

fn has_next_page(link: Option<&header::HeaderValue>) -> bool {
    link.and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .any(|part| part.contains("rel=\"next\"") || part.contains("rel=next"))
        })
}

#[derive(Debug)]
enum GitHubValue {
    NotModified,
    Value { value: Value, etag: Option<String> },
}

fn github_http_client(connect_timeout: Duration, request_timeout: Duration) -> AppResult<Client> {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(connect_timeout)
        .timeout(request_timeout)
        .user_agent("ai-center/0.1")
        .default_headers({
            let mut headers = header::HeaderMap::new();
            headers.insert(
                header::ACCEPT,
                header::HeaderValue::from_static("application/vnd.github+json"),
            );
            headers.insert(
                "x-github-api-version",
                header::HeaderValue::from_static(GITHUB_API_VERSION),
            );
            headers
        })
        .build()
        .map_err(|_| AppError::Connector("failed to build hardened GitHub client".into()))
}

fn value_only(value: GitHubValue) -> AppResult<Value> {
    match value {
        GitHubValue::Value { value, .. } => Ok(value),
        GitHubValue::NotModified => Err(AppError::Connector(
            "unexpected partial GitHub not-modified response".into(),
        )),
    }
}

fn required_string(value: &Value, pointer: &str, label: &str) -> AppResult<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| AppError::Connector(format!("GitHub response is missing {label}")))
}

fn required_git_oid(value: &Value, pointer: &str, label: &str) -> AppResult<String> {
    let oid = required_string(value, pointer, label)?;
    if valid_git_oid(&oid) {
        Ok(oid)
    } else {
        Err(AppError::Connector(format!(
            "GitHub response contains an invalid {label}"
        )))
    }
}

fn valid_git_oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn optional_datetime(value: &Value, pointer: &str) -> Option<DateTime<Utc>> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .and_then(|value| value.parse().ok())
}

#[cfg(test)]
mod tests;
