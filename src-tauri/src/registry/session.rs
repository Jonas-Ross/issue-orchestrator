use std::path::PathBuf;

use crate::pty::PtyHandles;

use super::status::Status;
use super::SessionSummary;

/// Single live session tracked by the registry. The `id` doubles as the
/// `ISSUE_ORCH_SESSION_ID` env var seeded into spawned `claude` processes
/// so hooks can report status back through the M3 socket.
pub struct Session {
    pub id: String,
    pub title: String,
    pub status: Status,
    pub handles: PtyHandles,
    pub claude_session_id: Option<String>,
    pub worktree_path: Option<PathBuf>,
    pub issue_url: Option<String>,
    pub branch: Option<String>,
    pub repo_name: Option<String>,
}

impl Session {
    /// Wire-friendly snapshot of this session. Single source of truth for
    /// shaping a `Session` into the IPC-visible `SessionSummary` — used
    /// by spawn, list, and the hook bridge's rebucket emit.
    pub fn to_summary(&self) -> SessionSummary {
        SessionSummary {
            id: self.id.clone(),
            title: self.title.clone(),
            status: self.status,
            worktree_path: self.worktree_path.as_ref().map(|p| p.display().to_string()),
            issue_url: self.issue_url.clone(),
            branch: self.branch.clone(),
            repo_name: self.repo_name.clone(),
        }
    }
}
