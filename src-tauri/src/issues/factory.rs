//! Construct the right `IssueClient` for a `RepoEntry` based on its
//! `IssueProvider`. Tokens for non-GitHub providers are read from the
//! macOS Keychain at construction time.

use std::sync::Arc;

use reqwest::Client;

use crate::config::{IssueProvider, RepoEntry};
use crate::error::Result;

use super::{
    github::GhCli,
    jira::{JiraClient, JiraConfig},
    linear::LinearClient,
    secrets, IssueClient,
};

pub fn make_client(repo: &RepoEntry, http: &Client) -> Result<Arc<dyn IssueClient>> {
    match &repo.provider {
        IssueProvider::Github => Ok(Arc::new(GhCli)),
        IssueProvider::Jira { base_url, email, project_key } => {
            let token = secrets::get_token("jira", &repo.name)?;
            Ok(Arc::new(JiraClient::new(
                http.clone(),
                JiraConfig {
                    base_url: base_url.clone(),
                    email: email.clone(),
                    project_key: project_key.clone(),
                    token,
                },
            )))
        }
        IssueProvider::Linear { team_key } => {
            let token = secrets::get_token("linear", &repo.name)?;
            Ok(Arc::new(LinearClient::new(http.clone(), team_key.clone(), token)))
        }
    }
}
