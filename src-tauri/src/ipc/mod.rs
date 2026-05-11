pub mod events;
pub mod issues;
pub mod pty;
pub mod repos;
pub mod secrets;
pub mod setup;

use std::sync::Arc;

use tauri::AppHandle;
use tauri_specta::Event;
use tokio::sync::mpsc;
use tracing::warn;

use crate::config::ConfigHandle;
use crate::registry::{RegistryCmd, RegistryEvent};
use crate::spawn::GitRunner;

/// Tauri-managed state — actor mailboxes (registry + config) and the git
/// runner. The issue client is constructed per-call by `issues::make_client`
/// from the target repo's `IssueProvider`, so different repos can use
/// different sources (GitHub / Jira / Linear) within one session.
pub struct AppState {
    pub registry: mpsc::Sender<RegistryCmd>,
    pub config: ConfigHandle,
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
        RegistryEvent::SessionUpdated(summary) => events::SessionUpdated(summary).emit(app),
        RegistryEvent::StatusChange { session_id, status } => {
            events::StatusChange { session_id, status }.emit(app)
        }
    }
}
