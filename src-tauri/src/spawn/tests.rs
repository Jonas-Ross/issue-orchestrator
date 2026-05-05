use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::config::{Config, RepoEntry};
use crate::error::{Error, Result};
use crate::registry::{RegistryCmd, SessionSummary, SpawnSpec};

use super::{
    parse_optimized_prompt, render_prompt, spawn_issue_session, GitRunner, Issue, IssueClient,
    DEFAULT_SPAWN_PROMPT,
};

struct StubIssueClient {
    issue: Issue,
}

#[async_trait]
impl IssueClient for StubIssueClient {
    async fn list(&self, _: &Path) -> Result<Vec<Issue>> {
        Ok(vec![self.issue.clone()])
    }
    async fn view(&self, _: &Path, _number: u64) -> Result<Issue> {
        Ok(self.issue.clone())
    }
    async fn body(&self, _: &Path, _number: u64) -> Result<String> {
        Ok(format!("body for #{}", self.issue.number))
    }
}

#[derive(Default)]
struct RecordingGit {
    branches_present: Vec<String>,
    worktrees_present: Vec<PathBuf>,
    calls: Mutex<Vec<String>>,
}

impl GitRunner for RecordingGit {
    fn branch_exists(&self, _repo: &Path, branch: &str) -> Result<bool> {
        Ok(self.branches_present.iter().any(|b| b == branch))
    }
    fn worktree_exists(&self, worktree_path: &Path) -> Result<bool> {
        Ok(self.worktrees_present.iter().any(|p| p == worktree_path))
    }
    fn worktree_add_new(
        &self,
        _repo: &Path,
        worktree_path: &Path,
        branch: &str,
    ) -> Result<()> {
        self.calls.lock().unwrap().push(format!(
            "add_new branch={branch} path={}",
            worktree_path.display()
        ));
        Ok(())
    }
    fn worktree_add_existing(
        &self,
        _repo: &Path,
        worktree_path: &Path,
        branch: &str,
    ) -> Result<()> {
        self.calls.lock().unwrap().push(format!(
            "add_existing branch={branch} path={}",
            worktree_path.display()
        ));
        Ok(())
    }
}

/// Fake registry: instead of running an actor, just answer Spawn replies
/// with a synthesized SessionSummary so we can inspect the request.
fn fake_registry(
    captured: Arc<Mutex<Option<SpawnSpec>>>,
) -> mpsc::Sender<RegistryCmd> {
    let (tx, mut rx) = mpsc::channel::<RegistryCmd>(8);
    tokio::spawn(async move {
        while let Some(cmd) = rx.recv().await {
            if let RegistryCmd::Spawn { spec, reply, .. } = cmd {
                let summary = SessionSummary {
                    id: "fake-session-id".into(),
                    title: match &spec {
                        SpawnSpec::Claude { title, .. } => title.clone(),
                        SpawnSpec::Bash => "bash".into(),
                    },
                    status: crate::registry::Status::Running,
                    worktree_path: match &spec {
                        SpawnSpec::Claude { worktree_path, .. } => {
                            Some(worktree_path.display().to_string())
                        }
                        SpawnSpec::Bash => None,
                    },
                    issue_url: match &spec {
                        SpawnSpec::Claude { issue_url, .. } => issue_url.clone(),
                        SpawnSpec::Bash => None,
                    },
                    branch: match &spec {
                        SpawnSpec::Claude { branch, .. } => branch.clone(),
                        SpawnSpec::Bash => None,
                    },
                    repo_name: match &spec {
                        SpawnSpec::Claude { repo_name, .. } => Some(repo_name.clone()),
                        SpawnSpec::Bash => None,
                    },
                };
                *captured.lock().unwrap() = Some(spec);
                let _ = reply.send(Ok(summary));
            }
        }
    });
    tx
}

fn temp_config() -> (tempfile::TempDir, Config) {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = Config {
        version: 1,
        worktree_root: dir.path().display().to_string(),
        repos: vec![RepoEntry {
            name: "demo".into(),
            path: "/dev/null".into(),
        }],
        spawn_prompt_template: None,
        setup_done: true,
    };
    (dir, config)
}

#[tokio::test]
async fn new_branch_path_uses_add_new() {
    let (_tmp, config) = temp_config();
    let issue_client = Arc::new(StubIssueClient {
        issue: Issue {
            number: 7,
            title: "Add tab strip".into(),
            labels: vec!["feat".into()],
            url: "https://github.com/demo/demo/issues/7".into(),
        },
    });
    let git = Arc::new(RecordingGit::default());
    let captured: Arc<Mutex<Option<SpawnSpec>>> = Arc::new(Mutex::new(None));
    let registry = fake_registry(Arc::clone(&captured));
    let repo = config.repos[0].clone();

    let summary = spawn_issue_session(
        &repo,
        7,
        &config,
        None,
        issue_client,
        git.clone(),
        registry,
        80,
        24,
    )
    .await
    .expect("spawn ok");

    assert_eq!(summary.title, "#7 Add tab strip");
    let calls = git.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 1);
    assert!(
        calls[0].starts_with("add_new branch=issue-7"),
        "expected add_new, got: {}",
        calls[0]
    );

    let spec = captured.lock().unwrap().take().expect("Spawn captured");
    match spec {
        SpawnSpec::Claude { cwd, prompt, issue_url, branch, repo_name, .. } => {
            assert!(cwd.ends_with("demo-issue-7"));
            assert!(prompt.contains("issue-team"));
            assert!(prompt.contains("#7"));
            assert_eq!(branch.as_deref(), Some("issue-7"));
            assert_eq!(repo_name, "demo");
            assert_eq!(
                issue_url.as_deref(),
                Some("https://github.com/demo/demo/issues/7")
            );
        }
        _ => panic!("expected Claude spec"),
    }

    assert_eq!(summary.repo_name.as_deref(), Some("demo"));
}

#[tokio::test]
async fn existing_branch_path_uses_add_existing() {
    let (_tmp, config) = temp_config();
    let issue_client = Arc::new(StubIssueClient {
        issue: Issue {
            number: 12,
            title: "Refactor registry".into(),
            labels: vec![],
            url: "https://github.com/demo/demo/issues/12".into(),
        },
    });
    let git = Arc::new(RecordingGit {
        branches_present: vec!["issue-12".into()],
        ..Default::default()
    });
    let captured: Arc<Mutex<Option<SpawnSpec>>> = Arc::new(Mutex::new(None));
    let registry = fake_registry(Arc::clone(&captured));
    let repo = config.repos[0].clone();

    spawn_issue_session(
        &repo,
        12,
        &config,
        None,
        issue_client,
        git.clone(),
        registry,
        80,
        24,
    )
    .await
    .expect("spawn ok");

    let calls = git.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 1);
    assert!(
        calls[0].starts_with("add_existing branch=issue-12"),
        "expected add_existing, got: {}",
        calls[0]
    );
}

#[tokio::test]
async fn existing_worktree_skips_git_add() {
    let (tmp, mut config) = temp_config();
    config.worktree_root = tmp.path().display().to_string();
    let wt = tmp.path().join("demo-issue-99");
    std::fs::create_dir_all(&wt).unwrap();

    let issue_client = Arc::new(StubIssueClient {
        issue: Issue {
            number: 99,
            title: "Reusable".into(),
            labels: vec![],
            url: "https://github.com/demo/demo/issues/99".into(),
        },
    });
    let git = Arc::new(RecordingGit {
        worktrees_present: vec![wt.clone()],
        ..Default::default()
    });
    let captured: Arc<Mutex<Option<SpawnSpec>>> = Arc::new(Mutex::new(None));
    let registry = fake_registry(Arc::clone(&captured));
    let repo = config.repos[0].clone();

    spawn_issue_session(
        &repo,
        99,
        &config,
        None,
        issue_client,
        git.clone(),
        registry,
        80,
        24,
    )
    .await
    .expect("spawn ok");

    let calls = git.calls.lock().unwrap().clone();
    assert!(
        calls.is_empty(),
        "no git worktree commands expected when worktree exists, got: {:?}",
        calls
    );
}

// ── Prompt rendering + override precedence ─────────────────────────────

#[test]
fn render_prompt_default_template() {
    let rendered = render_prompt(DEFAULT_SPAWN_PROMPT, 7, "Add tab strip");
    assert_eq!(
        rendered,
        "Use the issue-team skill to implement issue #7 (Add tab strip)."
    );
}

#[test]
fn render_prompt_custom_template_with_both_placeholders() {
    let rendered = render_prompt(
        "Implement {issue_title} (#{issue_number}) using feature-dev.",
        42,
        "Auth refactor",
    );
    assert_eq!(
        rendered,
        "Implement Auth refactor (#42) using feature-dev."
    );
}

#[test]
fn render_prompt_template_without_placeholders_passes_through() {
    let rendered = render_prompt("just do something", 1, "ignored");
    assert_eq!(rendered, "just do something");
}

#[tokio::test]
async fn prompt_override_takes_precedence_over_config_template() {
    let (_tmp, mut config) = temp_config();
    config.spawn_prompt_template = Some("config template #{issue_number}".into());

    let issue_client = Arc::new(StubIssueClient {
        issue: Issue {
            number: 5,
            title: "x".into(),
            labels: vec![],
            url: "https://github.com/demo/demo/issues/5".into(),
        },
    });
    let git = Arc::new(RecordingGit::default());
    let captured: Arc<Mutex<Option<SpawnSpec>>> = Arc::new(Mutex::new(None));
    let registry = fake_registry(Arc::clone(&captured));
    let repo = config.repos[0].clone();

    spawn_issue_session(
        &repo,
        5,
        &config,
        Some("override #{issue_number}".into()),
        issue_client,
        git,
        registry,
        80,
        24,
    )
    .await
    .expect("spawn ok");

    let spec = captured.lock().unwrap().take().expect("Spawn captured");
    match spec {
        SpawnSpec::Claude { prompt, .. } => assert_eq!(prompt, "override #5"),
        _ => panic!("expected Claude spec"),
    }
}

#[tokio::test]
async fn config_template_used_when_no_override() {
    let (_tmp, mut config) = temp_config();
    config.spawn_prompt_template =
        Some("Saved: {issue_title} (#{issue_number})".into());

    let issue_client = Arc::new(StubIssueClient {
        issue: Issue {
            number: 9,
            title: "Bug fix".into(),
            labels: vec![],
            url: "https://github.com/demo/demo/issues/9".into(),
        },
    });
    let git = Arc::new(RecordingGit::default());
    let captured: Arc<Mutex<Option<SpawnSpec>>> = Arc::new(Mutex::new(None));
    let registry = fake_registry(Arc::clone(&captured));
    let repo = config.repos[0].clone();

    spawn_issue_session(
        &repo,
        9,
        &config,
        None,
        issue_client,
        git,
        registry,
        80,
        24,
    )
    .await
    .expect("spawn ok");

    let spec = captured.lock().unwrap().take().expect("Spawn captured");
    match spec {
        SpawnSpec::Claude { prompt, .. } => assert_eq!(prompt, "Saved: Bug fix (#9)"),
        _ => panic!("expected Claude spec"),
    }
}

#[tokio::test]
async fn default_template_used_when_neither_override_nor_config() {
    let (_tmp, config) = temp_config(); // spawn_prompt_template = None

    let issue_client = Arc::new(StubIssueClient {
        issue: Issue {
            number: 3,
            title: "Hello".into(),
            labels: vec![],
            url: "https://github.com/demo/demo/issues/3".into(),
        },
    });
    let git = Arc::new(RecordingGit::default());
    let captured: Arc<Mutex<Option<SpawnSpec>>> = Arc::new(Mutex::new(None));
    let registry = fake_registry(Arc::clone(&captured));
    let repo = config.repos[0].clone();

    spawn_issue_session(
        &repo,
        3,
        &config,
        None,
        issue_client,
        git,
        registry,
        80,
        24,
    )
    .await
    .expect("spawn ok");

    let spec = captured.lock().unwrap().take().expect("Spawn captured");
    match spec {
        SpawnSpec::Claude { prompt, .. } => assert_eq!(
            prompt,
            "Use the issue-team skill to implement issue #3 (Hello)."
        ),
        _ => panic!("expected Claude spec"),
    }
}

// ── parse_optimized_prompt ─────────────────────────────────────────────

#[test]
fn parse_optimized_prompt_raw_json() {
    let p = parse_optimized_prompt(r#"{"prompt": "rewritten"}"#).unwrap();
    assert_eq!(p, "rewritten");
}

#[test]
fn parse_optimized_prompt_with_fence() {
    let raw = "```json\n{\"prompt\": \"fenced rewrite\"}\n```";
    assert_eq!(parse_optimized_prompt(raw).unwrap(), "fenced rewrite");
}

#[test]
fn parse_optimized_prompt_with_chatter() {
    let raw = "Sure! Here you go:\n{\"prompt\": \"ok\"}\nLet me know.";
    assert_eq!(parse_optimized_prompt(raw).unwrap(), "ok");
}

#[test]
fn parse_optimized_prompt_no_object_errors() {
    assert!(parse_optimized_prompt("nothing here").is_err());
}

#[test]
fn parse_optimized_prompt_empty_string_errors() {
    assert!(parse_optimized_prompt(r#"{"prompt": ""}"#).is_err());
}

// Make spawn::Error usable from tests
#[allow(dead_code)]
fn _force_error_use() -> Error {
    Error::Spawn("test".into())
}
