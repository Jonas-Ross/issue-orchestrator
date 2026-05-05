//! Construct the right `IssueClient` for a `RepoEntry` based on its
//! `IssueProvider`. Tokens for non-GitHub providers are read from the
//! macOS Keychain at construction time.

use std::sync::Arc;

use reqwest::Client;

use crate::config::{IssueProvider, RepoEntry};
use crate::error::Result;

use super::{github::GhCli, jira::JiraClient, linear::LinearClient, secrets, IssueClient};

pub fn make_client(repo: &RepoEntry) -> Result<Arc<dyn IssueClient>> {
    match &repo.provider {
        IssueProvider::Github => Ok(Arc::new(GhCli)),
        IssueProvider::Jira { base_url, email, project_key } => {
            let token = secrets::get_token("jira", &repo.name)?;
            Ok(Arc::new(JiraClient::new(
                Client::new(),
                base_url.clone(),
                email.clone(),
                project_key.clone(),
                token,
            )))
        }
        IssueProvider::Linear { team_key } => {
            let token = secrets::get_token("linear", &repo.name)?;
            Ok(Arc::new(LinearClient::new(
                Client::new(),
                team_key.clone(),
                token,
            )))
        }
    }
}
