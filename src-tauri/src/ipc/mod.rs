pub mod events;

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{AppHandle, State};
use tauri_specta::Event;
use tokio::sync::{mpsc, oneshot};
use tracing::warn;

use crate::config::Config;
use crate::registry::{RegistryCmd, RegistryEvent, SessionId, SessionSummary, SpawnSpec};

/// Tauri-managed state — actor mailbox plus paths/config the IPC layer
/// needs to expose to the frontend.
pub struct AppState {
    pub registry: mpsc::Sender<RegistryCmd>,
    pub config_path: PathBuf,
    pub hook_script_path: PathBuf,
    pub config: Mutex<Config>,
}

/// Bridge: drains domain events from the actor and re-emits them as the
/// matching typed Tauri events. Spawned once during app setup.
pub fn spawn_event_bridge(app: AppHandle, mut rx: mpsc::UnboundedReceiver<RegistryEvent>) {
    tokio::spawn(async move {
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

#[tauri::command]
#[specta::specta]
pub async fn pty_spawn(
    state: State<'_, AppState>,
    cols: u16,
    rows: u16,
) -> Result<SessionSummary, String> {
    let (tx, rx) = oneshot::channel();
    state
        .registry
        .send(RegistryCmd::Spawn {
            spec: SpawnSpec::Bash,
            cols,
            rows,
            reply: tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?.map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub async fn pty_write(
    state: State<'_, AppState>,
    id: SessionId,
    data: String,
) -> Result<(), String> {
    let (tx, rx) = oneshot::channel();
    state
        .registry
        .send(RegistryCmd::Write {
            id,
            data,
            reply: tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?.map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub async fn pty_resize(
    state: State<'_, AppState>,
    id: SessionId,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let (tx, rx) = oneshot::channel();
    state
        .registry
        .send(RegistryCmd::Resize {
            id,
            cols,
            rows,
            reply: tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?.map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub async fn pty_kill(state: State<'_, AppState>, id: SessionId) -> Result<(), String> {
    let (tx, rx) = oneshot::channel();
    state
        .registry
        .send(RegistryCmd::Kill { id, reply: tx })
        .await
        .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?.map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub async fn list_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<SessionSummary>, String> {
    let (tx, rx) = oneshot::channel();
    state
        .registry
        .send(RegistryCmd::List { reply: tx })
        .await
        .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SetupState {
    pub setup_done: bool,
    pub hook_script_path: String,
}

#[tauri::command]
#[specta::specta]
pub fn get_setup_state(state: State<'_, AppState>) -> Result<SetupState, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(SetupState {
        setup_done: config.setup_done,
        hook_script_path: state.hook_script_path.display().to_string(),
    })
}

#[tauri::command]
#[specta::specta]
pub fn mark_setup_done(state: State<'_, AppState>) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    if config.setup_done {
        return Ok(());
    }
    config.setup_done = true;
    config.save(&state.config_path).map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub fn get_config(state: State<'_, AppState>) -> Result<Config, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.clone())
}
