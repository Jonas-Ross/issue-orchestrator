//! Provider-agnostic issue source.
//!
//! Each repo can be backed by a different issue tracker (GitHub via the
//! `gh` CLI, Jira via REST v3, Linear via GraphQL). The trait below is
//! the boundary the spawn flow and IPC layer talk to; concrete impls
//! live in the per-provider submodules and are constructed on demand
//! by [`factory::make_client`] based on `RepoEntry.provider`.

use std::path::Path;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::Result;

pub mod factory;
pub mod github;
pub mod jira;
pub mod linear;
pub mod secrets;

#[cfg(test)]
mod tests;

pub use factory::make_client;

/// Provider-agnostic shape of an issue. The `id` is the human-readable
/// identifier used in branch names and prompt rendering: `"123"` for
/// GitHub, `"PROJ-123"` for Jira, `"ENG-456"` for Linear.
#[derive(Clone, Debug, Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub id: String,
    pub title: String,
    pub labels: Vec<String>,
    pub url: String,
}

/// Boundary trait for the per-repo issue source. Stored in IPC handlers
/// as `Arc<dyn IssueClient>`; tests substitute a stub.
#[async_trait]
pub trait IssueClient: Send + Sync {
    async fn list(&self, repo_path: &Path) -> Result<Vec<Issue>>;
    async fn view(&self, repo_path: &Path, id: &str) -> Result<Issue>;
    async fn body(&self, repo_path: &Path, id: &str) -> Result<String>;
}

/// Make a string safe for use as a git branch / worktree directory name.
/// Lowercases (so `PROJ-12` doesn't collide with `proj-12` on macOS's
/// default case-insensitive APFS volume), keeps `[a-z0-9._-]`, and
/// collapses runs of `-`. Leading/trailing `-` are trimmed.
pub fn sanitize_branch(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    let mut last_dash = false;
    for c in id.chars().flat_map(|c| c.to_lowercase()) {
        let keep = c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-';
        if keep && c != '-' {
            out.push(c);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "issue".to_owned()
    } else {
        trimmed.to_owned()
    }
}
