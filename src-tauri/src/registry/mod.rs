pub mod session;
pub mod status;

use std::collections::HashMap;
use std::path::PathBuf;

use portable_pty::CommandBuilder;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;
use tauri_specta::Event;
use tokio::sync::{mpsc, oneshot};
use tracing::{info, warn};
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::ipc::events::{PtyData, SessionAdded, SessionRemoved};
use crate::pty::{self, PtyEvent};

use self::session::Session;
pub use self::status::Status;

pub type SessionId = String;

/// Wire-friendly snapshot of a session — what the frontend gets back from
/// `pty_spawn`/`list_sessions` and inside `SessionAdded` events.
#[derive(Clone, Debug, Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: SessionId,
    pub title: String,
    pub status: Status,
    pub worktree_path: Option<String>,
}

/// What kind of process to launch. Phase 1 only spawns bash; Phase 3 (M4)
/// will add the `Claude` variant for issue-team sessions.
#[derive(Debug)]
pub enum SpawnSpec {
    Bash,
    Claude {
        cwd: PathBuf,
        prompt: String,
        worktree_path: PathBuf,
        title: String,
    },
}

/// Single entry point into the registry actor. Every state mutation goes
/// through one of these — no shared `Mutex<HashMap>`, no lock ordering to
/// reason about.
pub enum RegistryCmd {
    Spawn {
        spec: SpawnSpec,
        cols: u16,
        rows: u16,
        reply: oneshot::Sender<Result<SessionSummary>>,
    },
    Write {
        id: SessionId,
        data: String,
        reply: oneshot::Sender<Result<()>>,
    },
    Resize {
        id: SessionId,
        cols: u16,
        rows: u16,
        reply: oneshot::Sender<Result<()>>,
    },
    Kill {
        id: SessionId,
        reply: oneshot::Sender<Result<()>>,
    },
    List {
        reply: oneshot::Sender<Vec<SessionSummary>>,
    },
}

pub struct SessionRegistryActor {
    sessions: HashMap<SessionId, Session>,
    rx: mpsc::Receiver<RegistryCmd>,
    app: AppHandle,
}

impl SessionRegistryActor {
    /// Boot the actor on the Tauri tokio runtime and return the sender
    /// used by IPC commands and the M3 hook listener to drive it.
    pub fn spawn(app: AppHandle) -> mpsc::Sender<RegistryCmd> {
        let (tx, rx) = mpsc::channel(64);
        let actor = Self {
            sessions: HashMap::new(),
            rx,
            app,
        };
        tauri::async_runtime::spawn(actor.run());
        tx
    }

    async fn run(mut self) {
        info!("session registry actor started");
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                RegistryCmd::Spawn {
                    spec,
                    cols,
                    rows,
                    reply,
                } => {
                    let _ = reply.send(self.handle_spawn(spec, cols, rows));
                }
                RegistryCmd::Write { id, data, reply } => {
                    let _ = reply.send(self.handle_write(&id, &data));
                }
                RegistryCmd::Resize {
                    id,
                    cols,
                    rows,
                    reply,
                } => {
                    let _ = reply.send(self.handle_resize(&id, cols, rows));
                }
                RegistryCmd::Kill { id, reply } => {
                    let _ = reply.send(self.handle_kill(&id));
                }
                RegistryCmd::List { reply } => {
                    let _ = reply.send(self.snapshot());
                }
            }
        }
        info!("session registry actor stopped");
    }

    fn handle_spawn(
        &mut self,
        spec: SpawnSpec,
        cols: u16,
        rows: u16,
    ) -> Result<SessionSummary> {
        let id: SessionId = Uuid::new_v4().to_string();
        let (cmd, title, worktree_path) = build_command(&id, spec)?;

        let (tx_evt, rx_evt) = mpsc::channel::<PtyEvent>(256);
        let handles = pty::spawn_pty(cmd, cols, rows, tx_evt)?;

        let summary = SessionSummary {
            id: id.clone(),
            title: title.clone(),
            status: Status::Running,
            worktree_path: worktree_path.as_ref().map(|p| p.display().to_string()),
        };

        self.sessions.insert(
            id.clone(),
            Session {
                id: id.clone(),
                title,
                status: Status::Running,
                handles,
                claude_session_id: None,
                worktree_path,
            },
        );

        spawn_pty_forwarder(self.app.clone(), id.clone(), rx_evt);

        if let Err(e) = SessionAdded(summary.clone()).emit(&self.app) {
            warn!(?e, "failed to emit SessionAdded");
        }
        info!(session_id = %id, "session spawned");
        Ok(summary)
    }

    fn handle_write(&self, id: &str, data: &str) -> Result<()> {
        let s = self
            .sessions
            .get(id)
            .ok_or_else(|| Error::SessionNotFound(id.to_owned()))?;
        s.handles.write(data)
    }

    fn handle_resize(&self, id: &str, cols: u16, rows: u16) -> Result<()> {
        let s = self
            .sessions
            .get(id)
            .ok_or_else(|| Error::SessionNotFound(id.to_owned()))?;
        s.handles.resize(cols, rows)
    }

    fn handle_kill(&mut self, id: &str) -> Result<()> {
        match self.sessions.remove(id) {
            Some(_) => {
                if let Err(e) = (SessionRemoved {
                    session_id: id.to_owned(),
                })
                .emit(&self.app)
                {
                    warn!(?e, "failed to emit SessionRemoved");
                }
                info!(session_id = %id, "session killed");
                Ok(())
            }
            None => Err(Error::SessionNotFound(id.to_owned())),
        }
    }

    fn snapshot(&self) -> Vec<SessionSummary> {
        self.sessions
            .values()
            .map(|s| SessionSummary {
                id: s.id.clone(),
                title: s.title.clone(),
                status: s.status,
                worktree_path: s.worktree_path.as_ref().map(|p| p.display().to_string()),
            })
            .collect()
    }
}

/// Drain `PtyEvent`s from one PTY's reader thread and re-emit them to the
/// frontend as `pty:data` Tauri events. One forwarder task per session.
fn spawn_pty_forwarder(app: AppHandle, session_id: SessionId, mut rx: mpsc::Receiver<PtyEvent>) {
    tauri::async_runtime::spawn(async move {
        while let Some(evt) = rx.recv().await {
            match evt {
                PtyEvent::Data(chunk) => {
                    let _ = (PtyData {
                        session_id: session_id.clone(),
                        chunk,
                    })
                    .emit(&app);
                }
                PtyEvent::Eof => break,
            }
        }
    });
}

/// Translate a `SpawnSpec` into a portable-pty `CommandBuilder` plus
/// metadata for the registry. Always copies the parent env, sets
/// `TERM=xterm-256color`, and seeds `ISSUE_ORCH_SESSION_ID` so spawned
/// processes can be correlated back to a session via M3 hooks.
fn build_command(
    orch_id: &str,
    spec: SpawnSpec,
) -> Result<(CommandBuilder, String, Option<PathBuf>)> {
    match spec {
        SpawnSpec::Bash => {
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
            let mut cmd = CommandBuilder::new(shell);
            apply_common_env(&mut cmd, orch_id);
            if let Ok(home) = std::env::var("HOME") {
                cmd.cwd(home);
            }
            Ok((cmd, "bash".to_owned(), None))
        }
        SpawnSpec::Claude {
            cwd,
            prompt,
            worktree_path,
            title,
        } => {
            let mut cmd = CommandBuilder::new("claude");
            apply_common_env(&mut cmd, orch_id);
            cmd.cwd(&cwd);
            cmd.arg(prompt);
            Ok((cmd, title, Some(worktree_path)))
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
