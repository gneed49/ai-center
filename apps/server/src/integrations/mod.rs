pub mod github;
pub mod openai;

use std::sync::Arc;

use github::GitHubClient;

#[derive(Clone)]
pub struct GitHubRuntime {
    pub client: Arc<GitHubClient>,
    pub installation_id: u64,
}
