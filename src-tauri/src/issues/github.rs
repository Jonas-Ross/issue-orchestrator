//! GitHub issue source via the `gh` CLI. Identifiers are stringified
//! issue numbers (e.g. `"123"`).

use std::path::Path;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::process::Command;

use crate::error::{Error, Result};

use super::{Issue, IssueClient};

pub struct GhCli;

#[derive(Deserialize)]
struct GhIssue {
    number: u64,
    title: String,
    labels: Vec<GhLabel>,
    url: String,
}

#[derive(Deserialize)]
struct GhLabel {
    name: String,
}

impl From<GhIssue> for Issue {
    fn from(i: GhIssue) -> Self {
        Self {
            id: i.number.to_string(),
            title: i.title,
            labels: i.labels.into_iter().map(|l| l.name).collect(),
            url: i.url,
        }
    }
}

#[async_trait]
impl IssueClient for GhCli {
    async fn list(&self, repo_path: &Path) -> Result<Vec<Issue>> {
        let output = Command::new("gh")
            .args([
                "issue",
                "list",
                "--state",
                "open",
                "--limit",
                "50",
                "--json",
                "number,title,labels,url",
            ])
            .current_dir(repo_path)
            .output()
            .await
            .map_err(|e| Error::Issues(format!("gh: {e}")))?;
        if !output.status.success() {
            return Err(Error::Issues(format!(
                "gh issue list failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let parsed: Vec<GhIssue> = serde_json::from_slice(&output.stdout)
            .map_err(|e| Error::Issues(format!("gh json: {e}")))?;
        Ok(parsed.into_iter().map(Into::into).collect())
    }

    async fn view(&self, repo_path: &Path, id: &str) -> Result<Issue> {
        let output = Command::new("gh")
            .args(["issue", "view", id])
            .args(["--json", "number,title,labels,url"])
            .current_dir(repo_path)
            .output()
            .await
            .map_err(|e| Error::Issues(format!("gh: {e}")))?;
        if !output.status.success() {
            return Err(Error::Issues(format!(
                "gh issue view failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let parsed: GhIssue = serde_json::from_slice(&output.stdout)
            .map_err(|e| Error::Issues(format!("gh json: {e}")))?;
        Ok(parsed.into())
    }

    async fn body(&self, repo_path: &Path, id: &str) -> Result<String> {
        let output = Command::new("gh")
            .args(["issue", "view", id])
            .args(["--json", "body", "--jq", ".body"])
            .current_dir(repo_path)
            .output()
            .await
            .map_err(|e| Error::Issues(format!("gh: {e}")))?;
        if !output.status.success() {
            return Err(Error::Issues(format!(
                "gh issue view failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let body = String::from_utf8_lossy(&output.stdout).trim_end().to_owned();
        Ok(body)
    }
}
