use std::path::PathBuf;

use tauri::State;
use tokio::sync::oneshot;

use crate::config::{IssueProvider, RepoEntry};
use crate::registry::RegistryCmd;
use crate::spawn;

use super::AppState;

#[tauri::command]
#[specta::specta]
pub fn list_repos(state: State<'_, AppState>) -> Result<Vec<RepoEntry>, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.repos.clone())
}

#[tauri::command]
#[specta::specta]
pub fn add_repo(state: State<'_, AppState>, path: String) -> Result<RepoEntry, String> {
    let path = PathBuf::from(&path);
    spawn::validate_git_repo(&path).map_err(|e| e.to_string())?;
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    let entry = config.add_repo(&path).map_err(|e| e.to_string())?;
    config.save(&state.config_path).map_err(|e| e.to_string())?;
    Ok(entry)
}

#[tauri::command]
#[specta::specta]
pub async fn remove_repo(state: State<'_, AppState>, name: String) -> Result<(), String> {
    refuse_if_live_sessions(&state, &name).await?;
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.remove_repo(&name).map_err(|e| e.to_string())?;
    config.save(&state.config_path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Replace a repo's issue provider in-memory and on disk. Refuses while
/// the repo has live sessions — same guard as `remove_repo` — so an open
/// session can't end up with a stale `issue_url`/`branch` shape.
#[tauri::command]
#[specta::specta]
pub async fn update_repo_provider(
    state: State<'_, AppState>,
    repo_name: String,
    provider: IssueProvider,
) -> Result<RepoEntry, String> {
    refuse_if_live_sessions(&state, &repo_name).await?;
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    let entry = config
        .update_repo_provider(&repo_name, provider)
        .map_err(|e| e.to_string())?;
    config.save(&state.config_path).map_err(|e| e.to_string())?;
    Ok(entry)
}

/// Shared guard for the two mutators that would invalidate a running
/// session's metadata if the underlying repo changed underneath it.
async fn refuse_if_live_sessions(
    state: &State<'_, AppState>,
    repo_name: &str,
) -> Result<(), String> {
    let (tx, rx) = oneshot::channel();
    state
        .registry
        .send(RegistryCmd::List { reply: tx })
        .await
        .map_err(|e| e.to_string())?;
    let sessions = rx.await.map_err(|e| e.to_string())?;
    let live: Vec<String> = sessions
        .iter()
        .filter(|s| s.repo_name.as_deref() == Some(repo_name))
        .map(|s| s.id.clone())
        .collect();
    if live.is_empty() {
        return Ok(());
    }
    Err(format!(
        "Kill this repo's sessions first: {}",
        live.join(", ")
    ))
}
