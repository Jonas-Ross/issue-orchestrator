mod builder;
pub mod session;
pub mod status;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::sync::{mpsc, oneshot};
use tracing::{info, warn};
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::hooks::{HookEvent, NotificationKind};
use crate::pty::{self, PtyEvent};

use self::builder::build_command;
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
    pub issue_url: Option<String>,
    pub branch: Option<String>,
    /// Name of the repo this session belongs to. `None` for the Bash debug
    /// shell, which has no repo affinity. Used by the frontend to bucket
    /// sessions into per-repo drawers.
    pub repo_name: Option<String>,
}

/// What kind of process to launch. Phase 1 only spawns bash; Phase 3 (M4)
/// will use the `Claude` variant for issue-team sessions.
#[derive(Debug)]
pub enum SpawnSpec {
    Bash,
    Claude {
        cwd: PathBuf,
        prompt: String,
        worktree_path: PathBuf,
        title: String,
        issue_url: Option<String>,
        branch: Option<String>,
        repo_name: String,
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
    /// A Claude Code hook fired through the M3 UDS listener. Routed to
    /// the session whose id matches the hook's `session_orch_id`. Hooks
    /// for sessions we didn't spawn are silently dropped (no orch id
    /// means no correlation).
    HookEvent(HookEvent),
}

/// Domain events the actor publishes. A Tauri-aware bridge subscribes to
/// these and turns them into typed Tauri events for the frontend; tests
/// subscribe directly and assert against them. Keeping the actor free of
/// `AppHandle` is what makes it testable without a Tauri runtime.
#[derive(Clone, Debug)]
pub enum RegistryEvent {
    PtyData {
        session_id: SessionId,
        chunk: String,
    },
    SessionAdded(SessionSummary),
    SessionRemoved {
        session_id: SessionId,
    },
    StatusChange {
        session_id: SessionId,
        status: Status,
    },
}

pub struct SessionRegistryActor {
    sessions: HashMap<SessionId, Session>,
    rx: mpsc::Receiver<RegistryCmd>,
    events: mpsc::UnboundedSender<RegistryEvent>,
}

impl SessionRegistryActor {
    /// Boot the actor on the current tokio runtime and return its mailbox.
    /// `events` is the channel where domain events are published.
    pub fn spawn(events: mpsc::UnboundedSender<RegistryEvent>) -> mpsc::Sender<RegistryCmd> {
        let (tx, rx) = mpsc::channel(64);
        let actor = Self {
            sessions: HashMap::new(),
            rx,
            events,
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
                RegistryCmd::HookEvent(evt) => self.handle_hook(evt),
            }
        }
        info!("session registry actor stopped");
    }

    fn handle_hook(&mut self, evt: HookEvent) {
        let Some(orch_id) = evt.session_orch_id.as_deref() else {
            return;
        };
        let Some(session) = self.sessions.get_mut(orch_id) else {
            warn!(orch_id, event = %evt.hook_event_name, "hook for unknown session");
            return;
        };
        if let Some(claude_id) = &evt.claude_session_id {
            session.claude_session_id = Some(claude_id.clone());
        }
        let new_status = match evt.hook_event_name.as_str() {
            "SessionStart" => Status::Running,
            // Notification is overloaded: idle_prompt is the 60s reminder,
            // not a real "awaiting input" — must not pulse mint.
            "Notification" => match evt.notification_kind {
                Some(NotificationKind::IdlePrompt) => Status::Idle,
                _ => Status::NeedsInput,
            },
            "Stop" => Status::Idle,
            "SessionEnd" => Status::Exited,
            _ => return,
        };
        if session.status == new_status {
            return;
        }
        session.status = new_status;
        let session_id = session.id.clone();
        info!(session_id = %session_id, ?new_status, "hook updated session status");
        emit(
            &self.events,
            RegistryEvent::StatusChange {
                session_id,
                status: new_status,
            },
        );
    }

    fn handle_spawn(
        &mut self,
        spec: SpawnSpec,
        cols: u16,
        rows: u16,
    ) -> Result<SessionSummary> {
        let id: SessionId = Uuid::new_v4().to_string();
        let built = build_command(&id, spec)?;

        let (tx_evt, rx_evt) = mpsc::channel::<PtyEvent>(256);
        let handles = pty::spawn_pty(built.cmd, cols, rows, tx_evt)?;

        let summary = SessionSummary {
            id: id.clone(),
            title: built.title.clone(),
            status: Status::Running,
            worktree_path: built.worktree_path.as_ref().map(|p| p.display().to_string()),
            issue_url: built.issue_url.clone(),
            branch: built.branch.clone(),
            repo_name: built.repo_name.clone(),
        };

        self.sessions.insert(
            id.clone(),
            Session {
                id: id.clone(),
                title: built.title,
                status: Status::Running,
                handles,
                claude_session_id: None,
                worktree_path: built.worktree_path,
                issue_url: built.issue_url,
                branch: built.branch,
                repo_name: built.repo_name,
            },
        );

        spawn_pty_forwarder(self.events.clone(), id.clone(), rx_evt);
        emit(&self.events, RegistryEvent::SessionAdded(summary.clone()));
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
                emit(
                    &self.events,
                    RegistryEvent::SessionRemoved {
                        session_id: id.to_owned(),
                    },
                );
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
                issue_url: s.issue_url.clone(),
                branch: s.branch.clone(),
                repo_name: s.repo_name.clone(),
            })
            .collect()
    }
}

/// Drain `PtyEvent`s from one PTY's reader thread and re-publish them as
/// `RegistryEvent::PtyData`. One forwarder task per session.
fn spawn_pty_forwarder(
    events: mpsc::UnboundedSender<RegistryEvent>,
    session_id: SessionId,
    mut rx: mpsc::Receiver<PtyEvent>,
) {
    tauri::async_runtime::spawn(async move {
        while let Some(evt) = rx.recv().await {
            match evt {
                PtyEvent::Data(chunk) => {
                    if events
                        .send(RegistryEvent::PtyData {
                            session_id: session_id.clone(),
                            chunk,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
                PtyEvent::Eof => break,
            }
        }
    });
}

fn emit(events: &mpsc::UnboundedSender<RegistryEvent>, evt: RegistryEvent) {
    if let Err(e) = events.send(evt) {
        warn!(?e, "registry event channel closed");
    }
}

