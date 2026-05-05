use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};
use tracing::info;

use crate::config::{Config, RepoEntry};
use crate::error::{Error, Result};
use crate::registry::{RegistryCmd, SessionSummary, SpawnSpec};

#[cfg(test)]
mod tests;

/// Cheap check that a path looks like a git working copy. Accepts both
/// regular repos (`.git/` directory) and linked worktrees (`.git` file
/// pointing into the parent's worktrees dir). We don't shell out to
/// `git`: presence of `.git` is enough to fail fast on obvious mistakes
/// like adding a parent dir or a non-repo folder. Real `git` errors
/// later in the spawn flow surface their own messages.
pub fn validate_git_repo(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(Error::Config(format!("path not found: {}", path.display())));
    }
    if !path.is_dir() {
        return Err(Error::Config(format!("not a directory: {}", path.display())));
    }
    if !path.join(".git").exists() {
        return Err(Error::Config(format!(
            "not a git repo (no .git): {}",
            path.display()
        )));
    }
    Ok(())
}

#[derive(Clone, Debug, Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub labels: Vec<String>,
    pub url: String,
}

/// Boundary trait for the GitHub CLI. Stored in `AppState` as
/// `Arc<dyn IssueClient>` so tests can swap a mock in without touching
/// real `gh`.
#[async_trait]
pub trait IssueClient: Send + Sync {
    async fn list(&self, repo_path: &Path) -> Result<Vec<Issue>>;
    async fn view(&self, repo_path: &Path, number: u64) -> Result<Issue>;
    async fn body(&self, repo_path: &Path, number: u64) -> Result<String>;
}

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
            number: i.number,
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
            .map_err(|e| Error::Spawn(format!("gh: {e}")))?;
        if !output.status.success() {
            return Err(Error::Spawn(format!(
                "gh issue list failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let parsed: Vec<GhIssue> = serde_json::from_slice(&output.stdout)
            .map_err(|e| Error::Spawn(format!("gh json: {e}")))?;
        Ok(parsed.into_iter().map(Into::into).collect())
    }

    async fn view(&self, repo_path: &Path, number: u64) -> Result<Issue> {
        let output = Command::new("gh")
            .args(["issue", "view", &number.to_string()])
            .args(["--json", "number,title,labels,url"])
            .current_dir(repo_path)
            .output()
            .await
            .map_err(|e| Error::Spawn(format!("gh: {e}")))?;
        if !output.status.success() {
            return Err(Error::Spawn(format!(
                "gh issue view failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let parsed: GhIssue = serde_json::from_slice(&output.stdout)
            .map_err(|e| Error::Spawn(format!("gh json: {e}")))?;
        Ok(parsed.into())
    }

    async fn body(&self, repo_path: &Path, number: u64) -> Result<String> {
        let output = Command::new("gh")
            .args(["issue", "view", &number.to_string()])
            .args(["--json", "body", "--jq", ".body"])
            .current_dir(repo_path)
            .output()
            .await
            .map_err(|e| Error::Spawn(format!("gh: {e}")))?;
        if !output.status.success() {
            return Err(Error::Spawn(format!(
                "gh issue view failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let body = String::from_utf8_lossy(&output.stdout).trim_end().to_owned();
        Ok(body)
    }
}

/// Boundary trait for `git`. `GitCli` is the production impl; tests
/// substitute a recording mock.
pub trait GitRunner: Send + Sync {
    fn branch_exists(&self, repo: &Path, branch: &str) -> Result<bool>;
    fn worktree_exists(&self, worktree_path: &Path) -> Result<bool>;
    fn worktree_add_new(&self, repo: &Path, worktree_path: &Path, branch: &str) -> Result<()>;
    fn worktree_add_existing(&self, repo: &Path, worktree_path: &Path, branch: &str)
        -> Result<()>;
}

pub struct GitCli;

impl GitRunner for GitCli {
    fn branch_exists(&self, repo: &Path, branch: &str) -> Result<bool> {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["rev-parse", "--verify", "--quiet"])
            .arg(branch)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| Error::Spawn(format!("git: {e}")))?;
        Ok(status.success())
    }

    fn worktree_exists(&self, worktree_path: &Path) -> Result<bool> {
        Ok(worktree_path.is_dir())
    }

    fn worktree_add_new(&self, repo: &Path, worktree_path: &Path, branch: &str) -> Result<()> {
        run_git(
            repo,
            &["worktree", "add"],
            &[worktree_path.as_os_str(), "-b".as_ref(), branch.as_ref()],
        )
    }

    fn worktree_add_existing(
        &self,
        repo: &Path,
        worktree_path: &Path,
        branch: &str,
    ) -> Result<()> {
        run_git(
            repo,
            &["worktree", "add"],
            &[worktree_path.as_os_str(), branch.as_ref()],
        )
    }
}

fn run_git(repo: &Path, leading: &[&str], trailing: &[&std::ffi::OsStr]) -> Result<()> {
    let mut cmd = std::process::Command::new("git");
    cmd.arg("-C").arg(repo).args(leading);
    for arg in trailing {
        cmd.arg(arg);
    }
    let out = cmd
        .output()
        .map_err(|e| Error::Spawn(format!("git: {e}")))?;
    if !out.status.success() {
        return Err(Error::Spawn(format!(
            "git {leading:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    Ok(())
}

/// Built-in fallback prompt used when neither a per-session override
/// nor a saved `Config.spawn_prompt_template` is present. Mirrored on
/// the frontend in `src/lib/spawn-prompt.ts` — keep them in sync.
pub const DEFAULT_SPAWN_PROMPT: &str =
    "Use the issue-team skill to implement issue #{issue_number} ({issue_title}).";

/// Render a prompt template by replacing the two supported placeholders
/// literally. Templates with no placeholders pass through unchanged so
/// the user can opt out of interpolation entirely.
pub fn render_prompt(template: &str, issue_number: u64, issue_title: &str) -> String {
    template
        .replace("{issue_number}", &issue_number.to_string())
        .replace("{issue_title}", issue_title)
}

/// End-to-end spawn flow for an `issue-team` session:
///
/// 1. Look up the issue title via the issue client.
/// 2. Compute `<worktree_root>/<repo_name>-issue-<n>` and the
///    `issue-<n>` branch name.
/// 3. Reuse worktree if already present; else `git worktree add` against
///    the existing branch (if any) or with `-b` (new branch).
/// 4. Send `RegistryCmd::Spawn` for a `Claude` PTY in that cwd, with
///    `ISSUE_ORCH_SESSION_ID` seeded by the registry from its UUID.
///
/// Prompt resolution order: `prompt_override` → `config.spawn_prompt_template`
/// → `DEFAULT_SPAWN_PROMPT`. The chosen template is then rendered via
/// `render_prompt` to interpolate `{issue_number}` / `{issue_title}`.
pub async fn spawn_issue_session(
    repo: &RepoEntry,
    issue_number: u64,
    config: &Config,
    prompt_override: Option<String>,
    issue_client: Arc<dyn IssueClient>,
    git: Arc<dyn GitRunner>,
    registry: mpsc::Sender<RegistryCmd>,
    cols: u16,
    rows: u16,
) -> Result<SessionSummary> {
    let repo_path = PathBuf::from(&repo.path);
    let issue = issue_client.view(&repo_path, issue_number).await?;

    let branch = format!("issue-{issue_number}");
    let worktree_root = config.worktree_root_expanded();
    let wt_name = format!("{}-issue-{}", repo.name, issue_number);
    let worktree_path = worktree_root.join(&wt_name);

    if !git.worktree_exists(&worktree_path)? {
        std::fs::create_dir_all(&worktree_root)?;
        if git.branch_exists(&repo_path, &branch)? {
            info!(branch, path = %worktree_path.display(), "reusing existing branch in new worktree");
            git.worktree_add_existing(&repo_path, &worktree_path, &branch)?;
        } else {
            info!(branch, path = %worktree_path.display(), "creating new branch + worktree");
            git.worktree_add_new(&repo_path, &worktree_path, &branch)?;
        }
    } else {
        info!(path = %worktree_path.display(), "reusing existing worktree");
    }

    let template = prompt_override
        .as_deref()
        .or(config.spawn_prompt_template.as_deref())
        .unwrap_or(DEFAULT_SPAWN_PROMPT);
    let prompt = render_prompt(template, issue.number, &issue.title);
    let title = format!("#{} {}", issue.number, truncate(&issue.title, 40));

    let (reply_tx, reply_rx) = oneshot::channel();
    registry
        .send(RegistryCmd::Spawn {
            spec: SpawnSpec::Claude {
                cwd: worktree_path.clone(),
                prompt,
                worktree_path,
                title,
                issue_url: Some(issue.url.clone()),
                branch: Some(branch.clone()),
                repo_name: repo.name.clone(),
            },
            cols,
            rows,
            reply: reply_tx,
        })
        .await
        .map_err(|e| Error::Spawn(e.to_string()))?;

    reply_rx
        .await
        .map_err(|e| Error::Spawn(e.to_string()))?
}

fn truncate(s: &str, max_len: usize) -> String {
    let count = s.chars().count();
    if count <= max_len {
        return s.to_owned();
    }
    let mut out: String = s.chars().take(max_len.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Output of the headless "Decide next task" agent. Returned to the
/// frontend so the picker can highlight the recommendation and surface
/// the model's one-line reasoning.
#[derive(Clone, Debug, Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    pub number: u64,
    pub reasoning: String,
}

/// Run a one-shot `claude -p` over the open issues and ask the model to
/// pick the best one to tackle next. The prompt instructs the model to
/// respond with a single JSON object so we can parse it deterministically.
pub async fn decide_next_issue(
    repo: &RepoEntry,
    issue_client: Arc<dyn IssueClient>,
) -> Result<Decision> {
    let repo_path = PathBuf::from(&repo.path);
    let issues = issue_client.list(&repo_path).await?;
    if issues.is_empty() {
        return Err(Error::Spawn("no open issues to choose from".into()));
    }

    let prompt = build_decide_prompt(&issues);

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        Command::new("claude")
            .arg("-p")
            .arg(&prompt)
            .current_dir(&repo_path)
            .output(),
    )
    .await
    .map_err(|_| Error::Spawn("claude -p timed out after 60s".into()))?
    .map_err(|e| Error::Spawn(format!("claude: {e}")))?;

    if !output.status.success() {
        return Err(Error::Spawn(format!(
            "claude -p failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let decision = parse_decision(&stdout)?;

    if !issues.iter().any(|i| i.number == decision.number) {
        return Err(Error::Spawn(format!(
            "model picked #{} but it is not in the open issue list",
            decision.number
        )));
    }
    Ok(decision)
}

fn build_decide_prompt(issues: &[Issue]) -> String {
    let mut lines = String::new();
    for i in issues {
        let labels = if i.labels.is_empty() {
            String::new()
        } else {
            format!(" (labels: {})", i.labels.join(", "))
        };
        lines.push_str(&format!("- #{} — {}{}\n", i.number, i.title, labels));
    }
    format!(
        "You are picking the best GitHub issue to work on next from the list below.\n\
         Output ONLY a single JSON object — no preamble, no fenced code block, no commentary.\n\
         Schema: {{\"number\": <issue number>, \"reasoning\": \"<one short sentence>\"}}\n\n\
         Issues:\n{lines}"
    )
}

/// Run a one-shot `claude -p` inside the chosen repo so the model can
/// inspect the skills, plugins and MCPs available to a session there,
/// then ask it to rewrite `current_prompt` to take better advantage of
/// them. Returns the rewritten template (placeholders preserved).
pub async fn optimize_spawn_prompt(
    repo: &RepoEntry,
    current_prompt: &str,
) -> Result<String> {
    let repo_path = PathBuf::from(&repo.path);
    let prompt = build_optimize_prompt(current_prompt);

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        Command::new("claude")
            .arg("-p")
            .arg(&prompt)
            .current_dir(&repo_path)
            .output(),
    )
    .await
    .map_err(|_| Error::Spawn("claude -p timed out after 60s".into()))?
    .map_err(|e| Error::Spawn(format!("claude: {e}")))?;

    if !output.status.success() {
        return Err(Error::Spawn(format!(
            "claude -p failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_optimized_prompt(&stdout)
}

fn build_optimize_prompt(current_prompt: &str) -> String {
    format!(
        "You are improving a prompt template that will spawn a Claude Code session to implement \
         a GitHub issue. The session runs inside this repository, so you have access to its \
         skills, plugins, MCPs, hooks and CLAUDE.md.\n\n\
         Inspect what's available in this session and rewrite the template to make better use of \
         the most relevant skills (e.g. issue-team, feature-dev), tools, or MCPs you find. \
         Preserve the literal placeholders `{{issue_number}}` and `{{issue_title}}` somewhere in \
         the output so they can be interpolated at spawn time. Keep the result a single \
         template string — do not split it into multiple instructions or steps.\n\n\
         Output ONLY a single JSON object — no preamble, no fenced code block, no commentary.\n\
         Schema: {{\"prompt\": \"<rewritten template>\"}}\n\n\
         Current template:\n{current_prompt}"
    )
}

/// Tolerantly extract a rewritten prompt from `claude -p` stdout. Same
/// fence/chatter tolerance as `parse_decision`.
pub fn parse_optimized_prompt(raw: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct OptimizedPrompt {
        prompt: String,
    }
    let trimmed = raw.trim();
    let cleaned = strip_fence(trimmed);
    let json_str = extract_first_object(cleaned)
        .ok_or_else(|| Error::Spawn(format!("no JSON object in claude output: {raw}")))?;
    let parsed: OptimizedPrompt = serde_json::from_str(json_str)
        .map_err(|e| Error::Spawn(format!("parse optimized prompt json: {e} (input: {json_str})")))?;
    if parsed.prompt.trim().is_empty() {
        return Err(Error::Spawn("model returned empty prompt".into()));
    }
    Ok(parsed.prompt)
}

/// Tolerantly extract a `Decision` from `claude -p` stdout. The model is
/// asked to emit raw JSON, but we strip ``` fences and locate the first
/// `{...}` block defensively in case it adds chatter.
pub fn parse_decision(raw: &str) -> Result<Decision> {
    let trimmed = raw.trim();
    let cleaned = strip_fence(trimmed);
    let json_str = extract_first_object(cleaned)
        .ok_or_else(|| Error::Spawn(format!("no JSON object in claude output: {raw}")))?;
    serde_json::from_str::<Decision>(json_str)
        .map_err(|e| Error::Spawn(format!("parse decision json: {e} (input: {json_str})")))
}

fn strip_fence(s: &str) -> &str {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("```json") {
        return rest.trim_start_matches('\n').trim_end_matches("```").trim();
    }
    if let Some(rest) = s.strip_prefix("```") {
        return rest.trim_start_matches('\n').trim_end_matches("```").trim();
    }
    s
}

fn extract_first_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let mut depth = 0i32;
    for (i, c) in s[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let end = start + i + c.len_utf8();
                    return Some(&s[start..end]);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod decide_tests {
    use super::*;

    #[test]
    fn parses_raw_json() {
        let d = parse_decision(r#"{"number": 12, "reasoning": "small + isolated"}"#).unwrap();
        assert_eq!(d.number, 12);
        assert_eq!(d.reasoning, "small + isolated");
    }

    #[test]
    fn parses_json_inside_code_fence() {
        let raw = "```json\n{\"number\": 5, \"reasoning\": \"oldest open\"}\n```";
        let d = parse_decision(raw).unwrap();
        assert_eq!(d.number, 5);
    }

    #[test]
    fn parses_json_with_chatter() {
        let raw = "Sure! Here's my pick:\n{\"number\": 42, \"reasoning\": \"unblocks others\"}\nLet me know.";
        let d = parse_decision(raw).unwrap();
        assert_eq!(d.number, 42);
    }

    #[test]
    fn errors_with_no_object() {
        assert!(parse_decision("nothing here").is_err());
    }
}
