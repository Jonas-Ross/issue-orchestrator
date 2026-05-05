use std::path::PathBuf;

use portable_pty::CommandBuilder;

use crate::error::Result;

use super::SpawnSpec;

pub(super) struct BuiltCommand {
    pub cmd: CommandBuilder,
    pub title: String,
    pub worktree_path: Option<PathBuf>,
    pub issue_url: Option<String>,
    pub branch: Option<String>,
    pub repo_name: Option<String>,
}

/// Translate a `SpawnSpec` into a portable-pty `CommandBuilder` plus
/// metadata. Always copies the parent env, sets `TERM=xterm-256color`,
/// and seeds `ISSUE_ORCH_SESSION_ID` so spawned processes can be
/// correlated back to a session via M3 hooks.
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
    }
}

fn apply_common_env(cmd: &mut CommandBuilder, orch_id: &str) {
    for (k, v) in std::env::vars() {
        cmd.env(k, v);
    }
    cmd.env("TERM", "xterm-256color");
    cmd.env("ISSUE_ORCH_SESSION_ID", orch_id);
}
