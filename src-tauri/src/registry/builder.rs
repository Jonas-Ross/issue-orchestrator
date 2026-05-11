use std::path::PathBuf;

use portable_pty::CommandBuilder;

use crate::error::Result;

use super::SpawnSpec;

/// Title shown in the sidebar for a scratch `claude` session. Single
/// source so the spawn path (`ipc::pty::claude_spawn`) and the
/// cwd-inferred rebucket (`registry::handle_hook`) can't drift.
pub fn claude_title(repo: Option<&str>) -> String {
    match repo {
        Some(name) => format!("Claude · {name}"),
        None => "Claude".to_owned(),
    }
}

pub(super) struct BuiltCommand {
    pub cmd: CommandBuilder,
    pub title: String,
    pub worktree_path: Option<PathBuf>,
    pub issue_url: Option<String>,
    pub branch: Option<String>,
    pub repo_name: Option<String>,
}

/// Translate a `SpawnSpec` into a portable-pty `CommandBuilder` plus
/// metadata. Inherits the parent env (filtered through `should_drop_env`),
/// sets `TERM=xterm-256color`, and seeds `ISSUE_ORCH_SESSION_ID` so
/// hooks fired by the spawned `claude` can correlate back to a session.
pub(super) fn build_command(orch_id: &str, spec: SpawnSpec) -> Result<BuiltCommand> {
    match spec {
        SpawnSpec::Bash => {
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
            let mut cmd = CommandBuilder::new(shell);
            apply_common_env(&mut cmd, orch_id);
            if let Ok(home) = std::env::var("HOME") {
                cmd.cwd(home);
            }
            Ok(BuiltCommand {
                cmd,
                title: "bash".to_owned(),
                worktree_path: None,
                issue_url: None,
                branch: None,
                repo_name: None,
            })
        }
        SpawnSpec::Claude {
            cwd,
            prompt,
            worktree_path,
            title,
            issue_url,
            branch,
            repo_name,
        } => {
            let mut cmd = CommandBuilder::new("claude");
            apply_common_env(&mut cmd, orch_id);
            cmd.cwd(&cwd);
            cmd.arg(prompt);
            Ok(BuiltCommand {
                cmd,
                title,
                worktree_path: Some(worktree_path),
                issue_url,
                branch,
                repo_name: Some(repo_name),
            })
        }
        SpawnSpec::ClaudeAdHoc {
            cwd,
            title,
            repo_name,
        } => {
            let mut cmd = CommandBuilder::new("claude");
            apply_common_env(&mut cmd, orch_id);
            cmd.cwd(&cwd);
            Ok(BuiltCommand {
                cmd,
                title,
                worktree_path: None,
                issue_url: None,
                branch: None,
                repo_name,
            })
        }
    }
}

fn apply_common_env(cmd: &mut CommandBuilder, orch_id: &str) {
    for (k, v) in std::env::vars() {
        if should_drop_env(&k) {
            continue;
        }
        cmd.env(k, v);
    }
    cmd.env("TERM", "xterm-256color");
    cmd.env("ISSUE_ORCH_SESSION_ID", orch_id);
}

/// Exact env-var names dropped before a child PTY inherits them.
const EXACT_DROPS: &[&str] = &[
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
    "GOOGLE_APPLICATION_CREDENTIALS",
];

/// Suffix-matched (case-insensitive) drops. `_SECRET` is broad — it
/// will catch `STRIPE_SECRET`, `WEBHOOK_SECRET`, etc., which is
/// intentional. If a workflow-relevant credential ever ends in `_SECRET`
/// it would need to be explicitly allowed.
const SUFFIX_DROPS: &[&str] = &["_PASSWORD", "_PASSWD", "_SECRET"];

/// Drop credential-shaped env vars from the inherited env so a session
/// PTY can't see (and a misbehaving subprocess can't exfiltrate) the
/// dev's cloud secrets, DB passwords, etc. Workflow-relevant API keys
/// (ANTHROPIC_*, OPENAI_*, JIRA_*, GH_TOKEN, GITHUB_TOKEN) are kept —
/// dropping those would break the user's `gh` / `claude` setup.
fn should_drop_env(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    if EXACT_DROPS.contains(&upper.as_str()) {
        return true;
    }
    if SUFFIX_DROPS.iter().any(|s| upper.ends_with(s)) {
        return true;
    }
    upper.starts_with("GCP_") && upper.ends_with("_KEY")
}

#[cfg(test)]
mod tests {
    use super::should_drop_env;

    #[test]
    fn drops_aws_secret_triple() {
        assert!(should_drop_env("AWS_ACCESS_KEY_ID"));
        assert!(should_drop_env("AWS_SECRET_ACCESS_KEY"));
        assert!(should_drop_env("AWS_SESSION_TOKEN"));
    }

    #[test]
    fn drops_google_application_credentials() {
        assert!(should_drop_env("GOOGLE_APPLICATION_CREDENTIALS"));
    }

    #[test]
    fn drops_password_suffix_variants() {
        assert!(should_drop_env("DB_PASSWORD"));
        assert!(should_drop_env("MYSQL_PASSWD"));
        assert!(should_drop_env("REDIS_SECRET"));
    }

    #[test]
    fn drops_gcp_keys() {
        assert!(should_drop_env("GCP_SA_KEY"));
    }

    #[test]
    fn keeps_workflow_keys() {
        assert!(!should_drop_env("ANTHROPIC_API_KEY"));
        assert!(!should_drop_env("OPENAI_API_KEY"));
        assert!(!should_drop_env("GITHUB_TOKEN"));
        assert!(!should_drop_env("GH_TOKEN"));
        assert!(!should_drop_env("JIRA_TOKEN"));
        assert!(!should_drop_env("PATH"));
        assert!(!should_drop_env("HOME"));
        assert!(!should_drop_env("SHELL"));
    }
}
