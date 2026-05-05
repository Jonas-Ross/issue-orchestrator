use std::path::Path;
use std::sync::Arc;

use reqwest::Client;
use serde_json::json;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::config::{IssueProvider, RepoEntry};
use crate::error::Error;

use super::factory::make_client;
use super::jira::{adf_to_text, JiraClient};
use super::linear::LinearClient;
use super::{sanitize_branch, IssueClient};

// ── sanitize_branch ────────────────────────────────────────────────────

#[test]
fn sanitize_branch_passes_through_numeric() {
    assert_eq!(sanitize_branch("123"), "123");
}

#[test]
fn sanitize_branch_lowercases_jira_keys() {
    assert_eq!(sanitize_branch("PROJ-456"), "proj-456");
}

#[test]
fn sanitize_branch_collapses_runs_and_trims() {
    assert_eq!(sanitize_branch("Weird/key:!"), "weird-key");
    assert_eq!(sanitize_branch("--ENG--12--"), "eng-12");
}

#[test]
fn sanitize_branch_keeps_dot_and_underscore() {
    assert_eq!(sanitize_branch("v1.2_beta"), "v1.2_beta");
}

#[test]
fn sanitize_branch_empty_input_returns_placeholder() {
    assert_eq!(sanitize_branch("///"), "issue");
}

// ── factory dispatch ───────────────────────────────────────────────────

#[test]
fn factory_returns_gh_for_github_provider() {
    let repo = RepoEntry {
        name: "alpha".into(),
        path: "/tmp/alpha".into(),
        provider: IssueProvider::Github,
    };
    // Just assert it doesn't error; the GhCli construction is infallible.
    assert!(make_client(&repo).is_ok());
}

#[test]
fn factory_errors_when_jira_token_missing() {
    let repo = RepoEntry {
        name: "no-such-repo-for-testing".into(),
        path: "/tmp/x".into(),
        provider: IssueProvider::Jira {
            base_url: "https://example.invalid".into(),
            email: "ada@example.com".into(),
            project_key: "PROJ".into(),
        },
    };
    // No Keychain entry exists for this fake name in the test env; on
    // macOS this is `NoEntry`, on Linux the stub returns Config error.
    // `dyn IssueClient` doesn't implement Debug so we can't `unwrap_err`.
    match make_client(&repo) {
        Err(Error::Config(_)) => {}
        Err(other) => panic!("unexpected error variant: {other:?}"),
        Ok(_) => panic!("expected an error"),
    }
}

// ── Jira: ADF rendering ────────────────────────────────────────────────

#[test]
fn adf_text_node_unwraps() {
    let v = json!({ "type": "text", "text": "hello" });
    assert_eq!(adf_to_text(Some(&v)), "hello");
}

#[test]
fn adf_paragraph_with_inline_text() {
    let v = json!({
        "type": "doc",
        "content": [
            { "type": "paragraph", "content": [ { "type": "text", "text": "first line" } ] },
            { "type": "paragraph", "content": [ { "type": "text", "text": "second line" } ] }
        ]
    });
    let out = adf_to_text(Some(&v));
    assert!(out.contains("first line"));
    assert!(out.contains("second line"));
}

#[test]
fn adf_unknown_node_skipped() {
    let v = json!({
        "type": "doc",
        "content": [
            { "type": "weirdNode" },
            { "type": "paragraph", "content": [ { "type": "text", "text": "kept" } ] }
        ]
    });
    assert_eq!(adf_to_text(Some(&v)), "kept");
}

#[test]
fn adf_none_returns_empty() {
    assert_eq!(adf_to_text(None), "");
}

// ── Jira HTTP: list / view / body via wiremock ─────────────────────────

fn jira_with(server: &MockServer) -> JiraClient {
    JiraClient::new(
        Client::new(),
        server.uri(),
        "ada@example.com".into(),
        "PROJ".into(),
        "secret-token".into(),
    )
}

#[tokio::test]
async fn jira_list_parses_search_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/3/search"))
        .and(query_param("maxResults", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "issues": [
                { "key": "PROJ-1", "fields": { "summary": "First", "labels": ["bug"] } },
                { "key": "PROJ-2", "fields": { "summary": "Second", "labels": [] } }
            ]
        })))
        .mount(&server)
        .await;
    let client = jira_with(&server);
    let issues = client.list(Path::new("/")).await.unwrap();
    assert_eq!(issues.len(), 2);
    assert_eq!(issues[0].id, "PROJ-1");
    assert_eq!(issues[0].title, "First");
    assert_eq!(issues[0].labels, vec!["bug".to_string()]);
    assert_eq!(issues[0].url, format!("{}/browse/PROJ-1", server.uri()));
}

#[tokio::test]
async fn jira_view_uses_issue_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/3/issue/PROJ-7"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "key": "PROJ-7",
            "fields": { "summary": "Just one", "labels": ["feat"] }
        })))
        .mount(&server)
        .await;
    let client = jira_with(&server);
    let issue = client.view(Path::new("/"), "PROJ-7").await.unwrap();
    assert_eq!(issue.id, "PROJ-7");
    assert_eq!(issue.title, "Just one");
}

#[tokio::test]
async fn jira_body_renders_adf_to_text() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/3/issue/PROJ-9"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "key": "PROJ-9",
            "fields": {
                "summary": "ignored",
                "description": {
                    "type": "doc",
                    "content": [
                        { "type": "paragraph", "content": [ { "type": "text", "text": "Hello world" } ] }
                    ]
                }
            }
        })))
        .mount(&server)
        .await;
    let client = jira_with(&server);
    let body = client.body(Path::new("/"), "PROJ-9").await.unwrap();
    assert_eq!(body, "Hello world");
}

#[tokio::test]
async fn jira_sends_basic_auth_header() {
    let server = MockServer::start().await;
    // base64("ada@example.com:secret-token") = "YWRhQGV4YW1wbGUuY29tOnNlY3JldC10b2tlbg=="
    Mock::given(method("GET"))
        .and(path("/rest/api/3/search"))
        .and(header(
            "authorization",
            "Basic YWRhQGV4YW1wbGUuY29tOnNlY3JldC10b2tlbg==",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "issues": [] })))
        .mount(&server)
        .await;
    let client = jira_with(&server);
    client.list(Path::new("/")).await.unwrap();
}

#[tokio::test]
async fn jira_non_2xx_surfaces_status_in_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/3/search"))
        .respond_with(ResponseTemplate::new(401).set_body_string("nope"))
        .mount(&server)
        .await;
    let client = jira_with(&server);
    let err = client.list(Path::new("/")).await.unwrap_err();
    let msg = format!("{err:?}");
    assert!(msg.contains("401"), "expected status in error, got: {msg}");
}

// ── Linear HTTP: list / view via wiremock ──────────────────────────────

fn linear_with(server: &MockServer) -> LinearClient {
    LinearClient::new(Client::new(), "ENG".into(), "lin_api_key".into())
        .with_endpoint(format!("{}/graphql", server.uri()))
}

#[tokio::test]
async fn linear_list_parses_graphql_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "issues": {
                    "nodes": [
                        {
                            "identifier": "ENG-1",
                            "title": "First",
                            "url": "https://linear.app/x/issue/ENG-1",
                            "labels": { "nodes": [ { "name": "bug" } ] }
                        }
                    ]
                }
            }
        })))
        .mount(&server)
        .await;
    let client = linear_with(&server);
    let issues = client.list(Path::new("/")).await.unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].id, "ENG-1");
    assert_eq!(issues[0].labels, vec!["bug".to_string()]);
}

#[tokio::test]
async fn linear_view_extracts_single_issue() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "issue": {
                    "identifier": "ENG-7",
                    "title": "Lonely",
                    "url": "https://linear.app/x/issue/ENG-7",
                    "labels": { "nodes": [] }
                }
            }
        })))
        .mount(&server)
        .await;
    let client = linear_with(&server);
    let issue = client.view(Path::new("/"), "ENG-7").await.unwrap();
    assert_eq!(issue.id, "ENG-7");
    assert_eq!(issue.title, "Lonely");
}

#[tokio::test]
async fn linear_body_returns_description_string() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "issue": {
                    "identifier": "ENG-2",
                    "title": "x",
                    "url": "https://linear.app/x/issue/ENG-2",
                    "labels": { "nodes": [] },
                    "description": "markdown body here"
                }
            }
        })))
        .mount(&server)
        .await;
    let client = linear_with(&server);
    let body = client.body(Path::new("/"), "ENG-2").await.unwrap();
    assert_eq!(body, "markdown body here");
}

#[tokio::test]
async fn linear_surfaces_graphql_errors() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "errors": [ { "message": "rate limited" } ],
            "data": null
        })))
        .mount(&server)
        .await;
    let client = linear_with(&server);
    let err = client.list(Path::new("/")).await.unwrap_err();
    let msg = format!("{err:?}");
    assert!(msg.contains("rate limited"), "got: {msg}");
}

#[tokio::test]
async fn linear_sends_authorization_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(header("authorization", "lin_api_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "issues": { "nodes": [] } }
        })))
        .mount(&server)
        .await;
    let client = linear_with(&server);
    client.list(Path::new("/")).await.unwrap();
}

// Force `Arc<dyn IssueClient>` import to compile so the trait stays
// object-safe under churn.
#[allow(dead_code)]
fn _trait_object_check() -> Arc<dyn IssueClient> {
    Arc::new(super::github::GhCli)
}
