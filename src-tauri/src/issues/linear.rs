//! Linear issue source via GraphQL. Identifiers are Linear's
//! human-readable issue identifiers (e.g. `"ENG-123"`).

use std::path::Path;

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use crate::error::{Error, Result};

use super::{check_http_response, Issue, IssueClient};

const ENDPOINT: &str = "https://api.linear.app/graphql";

pub struct LinearClient {
    http: Client,
    endpoint: String,
    team_key: String,
    token: String,
}

impl LinearClient {
    pub fn new(http: Client, team_key: String, token: String) -> Self {
        Self { http, endpoint: ENDPOINT.into(), team_key, token }
    }

    #[cfg(test)]
    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        self.endpoint = endpoint;
        self
    }
}

#[derive(Deserialize)]
struct GqlEnvelope<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Option<Vec<GqlError>>,
}

#[derive(Deserialize)]
struct GqlError {
    message: String,
}

#[derive(Deserialize)]
struct ListData {
    issues: IssueConnection,
}

#[derive(Deserialize)]
struct IssueConnection {
    nodes: Vec<RawIssue>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawIssue {
    identifier: String,
    title: String,
    url: String,
    labels: LabelConnection,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Deserialize)]
struct LabelConnection {
    nodes: Vec<LabelNode>,
}

#[derive(Deserialize)]
struct LabelNode {
    name: String,
}

#[derive(Deserialize)]
struct ViewData {
    issue: RawIssue,
}

impl From<RawIssue> for Issue {
    fn from(r: RawIssue) -> Self {
        Self {
            id: r.identifier,
            title: r.title,
            url: r.url,
            labels: r.labels.nodes.into_iter().map(|n| n.name).collect(),
        }
    }
}

const ISSUE_FIELDS: &str = "identifier title url labels { nodes { name } }";

impl LinearClient {
    async fn execute<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: serde_json::Value,
        ctx: &str,
    ) -> Result<T> {
        let resp = self
            .http
            .post(&self.endpoint)
            .header("Authorization", &self.token)
            .header("Content-Type", "application/json")
            .json(&json!({ "query": query, "variables": variables }))
            .send()
            .await
            .map_err(|e| Error::Http(format!("linear: {e}")))?;
        let resp = check_http_response(resp, &format!("linear {ctx}")).await?;
        let env: GqlEnvelope<T> = resp
            .json()
            .await
            .map_err(|e| Error::Http(format!("linear json: {e}")))?;
        if let Some(errs) = env.errors.filter(|e| !e.is_empty()) {
            let joined = errs
                .into_iter()
                .map(|e| e.message)
                .collect::<Vec<_>>()
                .join("; ");
            return Err(Error::Http(format!("linear {ctx}: {joined}")));
        }
        env.data
            .ok_or_else(|| Error::Http(format!("linear {ctx}: empty data")))
    }
}

#[async_trait]
impl IssueClient for LinearClient {
    async fn list(&self, _repo_path: &Path) -> Result<Vec<Issue>> {
        let query = format!(
            "query($teamKey: String!) {{ \
                issues( \
                    first: 50, \
                    filter: {{ \
                        team: {{ key: {{ eq: $teamKey }} }}, \
                        state: {{ type: {{ neq: \"completed\" }} }} \
                    }} \
                ) {{ nodes {{ {ISSUE_FIELDS} }} }} \
            }}"
        );
        let data: ListData = self
            .execute(&query, json!({ "teamKey": self.team_key }), "list")
            .await?;
        Ok(data.issues.nodes.into_iter().map(Into::into).collect())
    }

    async fn view(&self, _repo_path: &Path, id: &str) -> Result<Issue> {
        let query = format!(
            "query($id: String!) {{ issue(id: $id) {{ {ISSUE_FIELDS} }} }}"
        );
        let data: ViewData = self
            .execute(&query, json!({ "id": id }), "view")
            .await?;
        Ok(data.issue.into())
    }

    async fn body(&self, _repo_path: &Path, id: &str) -> Result<String> {
        let query = format!(
            "query($id: String!) {{ issue(id: $id) {{ {ISSUE_FIELDS} description }} }}"
        );
        let data: ViewData = self
            .execute(&query, json!({ "id": id }), "body")
            .await?;
        Ok(data.issue.description.unwrap_or_default())
    }
}
