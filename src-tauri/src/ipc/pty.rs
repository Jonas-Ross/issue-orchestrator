use tauri::State;
use tokio::sync::oneshot;

use crate::registry::{RegistryCmd, SessionId, SessionSummary, SpawnSpec};

use super::AppState;

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
