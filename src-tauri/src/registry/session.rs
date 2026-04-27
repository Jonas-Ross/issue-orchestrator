use std::path::PathBuf;

use crate::pty::PtyHandles;

use super::status::Status;

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
}
