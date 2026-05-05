use std::path::Path;
use std::process::Stdio;

use crate::error::{Error, Result};

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
