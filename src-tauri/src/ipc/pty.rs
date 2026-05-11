use std::path::PathBuf;

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

/// Spawn a scratch `claude` session with no prompt arg. When
/// `repo_name` is set the session is bucketed under that repo's
/// drawer and starts in the repo path; otherwise it starts in `$HOME`
/// and lands in the unbucketed drawer. The cwd-inferred rebucket in
/// the hook bridge will move it later if the user `cd`s into a
/// tracked repo and the next hook event arrives.
#[tauri::command]
#[specta::specta]
pub async fn claude_spawn(
    state: State<'_, AppState>,
    repo_name: Option<String>,
    cols: u16,
    rows: u16,
) -> Result<SessionSummary, String> {
    let (cwd, title) = match &repo_name {
        Some(name) => {
            // Surface lookup errors directly: a name passed from the
            // frontend always comes from `repos.value`, so a miss is
            // a real bug worth seeing rather than masking with $HOME.
            let repo = state.config.lookup_repo(name).await.map_err(|e| e.to_string())?;
            (PathBuf::from(repo.path), format!("Claude · {name}"))
        }
        None => {
            let home = std::env::var("HOME")
                .map_err(|e| format!("HOME env var: {e}"))?;
            (PathBuf::from(home), "Claude".to_owned())
        }
    };

    let (tx, rx) = oneshot::channel();
    state
        .registry
        .send(RegistryCmd::Spawn {
            spec: SpawnSpec::ClaudeAdHoc {
                cwd,
                title,
                repo_name,
            },
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
