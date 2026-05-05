pub mod events;
pub mod issues;
pub mod pty;
pub mod repos;
pub mod secrets;
pub mod setup;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, State};
use tauri_specta::Event;
use tokio::sync::mpsc;
use tracing::warn;

use crate::config::{Config, RepoEntry};
use crate::registry::{RegistryCmd, RegistryEvent};
use crate::spawn::GitRunner;

/// Tauri-managed state — actor mailbox plus paths/config and the git
/// runner. The issue client is constructed per-call by `issues::make_client`
/// from the target repo's `IssueProvider`, so different repos can use
/// different sources (GitHub / Jira / Linear) within one session.
pub struct AppState {
    pub registry: mpsc::Sender<RegistryCmd>,
    pub config_path: PathBuf,
    pub config: Mutex<Config>,
    pub git_runner: Arc<dyn GitRunner>,
    /// Shared HTTP client. `reqwest::Client::new()` allocates a fresh
    /// connection pool and rustls context, so we build it once at app
    /// start and clone the cheap handle into each Jira/Linear client.
    pub http: reqwest::Client,
}

/// Bridge: drains domain events from the actor and re-emits them as the
/// matching typed Tauri events. Spawned once during app setup.
pub fn spawn_event_bridge(app: AppHandle, mut rx: mpsc::UnboundedReceiver<RegistryEvent>) {
    tauri::async_runtime::spawn(async move {
        while let Some(evt) = rx.recv().await {
            if let Err(e) = forward(&app, evt) {
                warn!(?e, "failed to emit Tauri event");
            }
        }
    });
}

fn forward(app: &AppHandle, evt: RegistryEvent) -> tauri::Result<()> {
    match evt {
        RegistryEvent::PtyData { session_id, chunk } => {
            events::PtyData { session_id, chunk }.emit(app)
        }
        RegistryEvent::SessionAdded(summary) => events::SessionAdded(summary).emit(app),
        RegistryEvent::SessionRemoved { session_id } => {
            events::SessionRemoved { session_id }.emit(app)
        }
        RegistryEvent::StatusChange { session_id, status } => {
            events::StatusChange { session_id, status }.emit(app)
        }
    }
}

/// Find a repo by name in the locked config, cloning it out so callers
/// don't hold the mutex across `.await`. Used by every command that
/// needs the full `RepoEntry` (issue commands, spawn). Removed in the
/// follow-up that converts `AppState.config` to a config actor.
pub(crate) fn lookup_repo(
    state: &State<'_, AppState>,
    repo_name: &str,
) -> Result<RepoEntry, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    config
        .repos
        .iter()
        .find(|r| r.name == repo_name)
        .cloned()
        .ok_or_else(|| format!("unknown repo: {repo_name}"))
}
