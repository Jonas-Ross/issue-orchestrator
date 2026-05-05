//! Jira issue source via REST v3. Identifiers are Jira issue keys
//! (e.g. `"PROJ-123"`).
//!
//! Auth: HTTP Basic with `email:token`. The token is read from the
//! Keychain by the factory and passed in the constructor — never logged
//! or returned over IPC.

use std::path::Path;

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use crate::error::{Error, Result};

use super::{check_http_response, Issue, IssueClient};

pub struct JiraConfig {
    pub base_url: String,
    pub email: String,
    pub project_key: String,
    pub token: String,
}

pub struct JiraClient {
    http: Client,
    cfg: JiraConfig,
}

impl JiraClient {
    pub fn new(http: Client, cfg: JiraConfig) -> Self {
        Self { http, cfg }
    }

    fn auth_header(&self) -> String {
        let raw = format!("{}:{}", self.cfg.email, self.cfg.token);
        format!("Basic {}", B64.encode(raw))
    }

    fn issue_url(&self, key: &str) -> String {
        format!("{}/browse/{key}", self.cfg.base_url.trim_end_matches('/'))
    }

    fn endpoint(&self, suffix: &str) -> String {
        format!("{}{suffix}", self.cfg.base_url.trim_end_matches('/'))
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        query: &[(&str, &str)],
        ctx: &str,
    ) -> Result<T> {
        let resp = self
            .http
            .get(url)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/json")
            .query(query)
            .send()
            .await
            .map_err(|e| Error::Spawn(format!("jira: {e}")))?;
        let resp = check_http_response(resp, ctx).await?;
        resp.json()
            .await
            .map_err(|e| Error::Spawn(format!("jira json: {e}")))
    }
}

#[derive(Deserialize)]
struct SearchResp {
    issues: Vec<RawIssue>,
}

#[derive(Deserialize)]
struct RawIssue {
    key: String,
    fields: RawFields,
}

#[derive(Deserialize)]
struct RawFields {
    summary: String,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    description: Option<Value>,
}

#[async_trait]
impl IssueClient for JiraClient {
    async fn list(&self, _repo_path: &Path) -> Result<Vec<Issue>> {
        let jql = format!(
            "project = {} AND statusCategory != Done ORDER BY updated DESC",
            self.cfg.project_key
        );
        let url = self.endpoint("/rest/api/3/search");
        let parsed: SearchResp = self
            .get_json(
                &url,
                &[
                    ("jql", jql.as_str()),
                    ("maxResults", "50"),
                    ("fields", "summary,labels"),
                ],
                "jira search",
            )
            .await?;
        Ok(parsed
            .issues
            .into_iter()
            .map(|r| Issue {
                url: self.issue_url(&r.key),
                id: r.key,
                title: r.fields.summary,
                labels: r.fields.labels,
            })
            .collect())
    }

    async fn view(&self, _repo_path: &Path, id: &str) -> Result<Issue> {
        let url = self.endpoint(&format!("/rest/api/3/issue/{id}"));
        let raw: RawIssue = self
            .get_json(&url, &[("fields", "summary,labels")], "jira view")
            .await?;
        Ok(Issue {
            url: self.issue_url(&raw.key),
            id: raw.key,
            title: raw.fields.summary,
            labels: raw.fields.labels,
        })
    }

    async fn body(&self, _repo_path: &Path, id: &str) -> Result<String> {
        let url = self.endpoint(&format!("/rest/api/3/issue/{id}"));
        let raw: RawIssue = self
            .get_json(&url, &[("fields", "description")], "jira body")
            .await?;
        Ok(adf_to_text(raw.fields.description.as_ref()))
    }
}

/// Walk Jira's Atlassian Document Format and return a plain-text
/// rendering. Only enough to surface the body in the picker preview —
/// we don't try to round-trip formatting. Unknown nodes are skipped;
/// text nodes are concatenated with appropriate paragraph breaks.
pub fn adf_to_text(node: Option<&Value>) -> String {
    let mut out = String::new();
    if let Some(v) = node {
        walk(v, &mut out);
    }
    out.trim().to_owned()
}

fn walk(node: &Value, out: &mut String) {
    match node.get("type").and_then(|t| t.as_str()) {
        Some("text") => {
            if let Some(t) = node.get("text").and_then(|t| t.as_str()) {
                out.push_str(t);
            }
        }
        Some("hardBreak") => out.push('\n'),
        Some("paragraph") | Some("heading") | Some("listItem") | Some("codeBlock") => {
            walk_children(node, out);
            out.push_str("\n\n");
        }
        Some("bulletList") | Some("orderedList") | Some("blockquote") => {
            walk_children(node, out);
        }
        _ => walk_children(node, out),
    }
}

fn walk_children(node: &Value, out: &mut String) {
    if let Some(arr) = node.get("content").and_then(|c| c.as_array()) {
        for child in arr {
            walk(child, out);
        }
    }
}
