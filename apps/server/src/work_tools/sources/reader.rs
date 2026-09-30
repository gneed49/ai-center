//! Three bounded Notion reads or one Linear query, never a remote mutation.
use super::{
    locator::SourceLocator,
    models::{ExistingReadErrorCode as Code, ExistingReadFailure as Failure, ExistingReadOutcome},
};
use reqwest::Client;
use secrecy::SecretString;
use std::time::Duration;

#[path = "linear.rs"]
mod linear;
#[path = "notion.rs"]
mod notion;
#[path = "transport.rs"]
mod transport;

pub struct ExistingToolReader {
    client: Client,
    loopback: Option<String>,
    deadline: Duration,
}
impl ExistingToolReader {
    pub fn official() -> Result<Self, Failure> {
        Self::build(None, Duration::from_secs(30), Duration::from_secs(45))
    }
    fn build(
        loopback: Option<String>,
        request_timeout: Duration,
        deadline: Duration,
    ) -> Result<Self, Failure> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(request_timeout)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .user_agent("AI-Center/1.0")
            .build()
            .map_err(|_| Failure::new(Code::RemoteTransport))?;
        Ok(Self {
            client,
            loopback,
            deadline,
        })
    }
    /// Local contract tests only: no alternate production origin can be injected.
    #[cfg(debug_assertions)]
    pub fn loopback(
        base: &str,
        request_timeout: Duration,
        deadline: Duration,
    ) -> Result<Self, Failure> {
        let url = url::Url::parse(base).map_err(|_| Failure::new(Code::RemoteResponseInvalid))?;
        if url.scheme() != "http"
            || url.host_str() != Some("127.0.0.1")
            || url.port().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
            || deadline.is_zero()
            || deadline > Duration::from_secs(45)
            || request_timeout.is_zero()
            || request_timeout > Duration::from_secs(30)
        {
            return Err(Failure::new(Code::RemoteResponseInvalid));
        }
        Self::build(
            Some(base.trim_end_matches('/').into()),
            request_timeout,
            deadline,
        )
    }
    pub async fn read_existing(
        &self,
        secret: &SecretString,
        locator: &SourceLocator,
    ) -> Result<ExistingReadOutcome, Failure> {
        // Service separately starts its 45 s admission/network window before DB admission.
        // This outer timer also bounds multi-request Notion and streaming bodies.
        let result = tokio::time::timeout(self.deadline, async {
            match locator {
                SourceLocator::NotionPage { id } => self.read_notion(secret, *id).await,
                SourceLocator::LinearIssue {
                    key,
                    expected_workspace_slug,
                } => {
                    self.read_linear(secret, key, expected_workspace_slug.as_deref())
                        .await
                }
            }
        })
        .await
        .map_err(|_| Failure::new(Code::RemoteTimeout))?;
        match result {
            Ok(snapshot) => Ok(ExistingReadOutcome::Available(Box::new(snapshot))),
            Err(failure)
                if matches!(
                    failure.code,
                    Code::RemoteAuthentication | Code::RemotePermission | Code::RemoteNotFound
                ) =>
            {
                Ok(ExistingReadOutcome::Unavailable { code: failure.code })
            }
            Err(failure) => Err(failure),
        }
    }
}

#[cfg(test)]
#[path = "reader_tests.rs"]
mod tests;
