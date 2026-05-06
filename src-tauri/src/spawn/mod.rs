use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};
use tracing::info;

use crate::config::{Config, RepoEntry};
use crate::error::{Error, Result};
use crate::issues::{sanitize_branch, IssueClient};
use crate::registry::{RegistryCmd, SessionSummary, SpawnSpec};

pub mod git;
pub mod headless;

#[cfg(test)]
mod tests;

pub use crate::issues::Issue;
pub use git::{validate_git_repo, GitCli, GitRunner};
pub use headless::{
    decide_next_issue, optimize_spawn_prompt, parse_decision, parse_optimized_prompt, Decision,
};

/// Built-in fallback prompt used when neither a per-session override
/// nor a saved `Config.spawn_prompt_template` is present. Mirrored on
/// the frontend in `src/lib/spawn-prompt.ts` — keep them in sync.
pub const DEFAULT_SPAWN_PROMPT: &str =
    "Use the issue-team skill to implement issue #{issue_id} ({issue_title}).";

/// Render a prompt template by replacing the supported placeholders
/// literally. `{issue_number}` is accepted as a back-compat alias for
/// `{issue_id}` so custom templates saved before the multi-provider
/// switch keep working. Templates with no placeholders pass through
/// unchanged so the user can opt out of interpolation entirely.
pub fn render_prompt(template: &str, issue_id: &str, issue_title: &str) -> String {
    template
        .replace("{issue_id}", issue_id)
        .replace("{issue_number}", issue_id)
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
    issue_id: String,
    config: &Config,
    prompt_override: Option<String>,
    issue_client: Arc<dyn IssueClient>,
    git: Arc<dyn GitRunner>,
    registry: mpsc::Sender<RegistryCmd>,
    cols: u16,
    rows: u16,
) -> Result<SessionSummary> {
    let repo_path = PathBuf::from(&repo.path);
    let issue = issue_client.view(&repo_path, &issue_id).await?;

    let safe = sanitize_branch(&issue.id);
    let branch = format!("issue-{safe}");
    let worktree_root = config.worktree_root_expanded();
    let wt_name = format!("{}-issue-{}", repo.name, safe);
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
    let prompt = render_prompt(template, &issue.id, &issue.title);
    let title = format!("#{} {}", issue.id, truncate(&issue.title, 40));

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
