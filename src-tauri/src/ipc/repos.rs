use std::path::PathBuf;

use tauri::State;
use tokio::sync::oneshot;

use crate::config::{IssueProvider, RepoEntry};
use crate::registry::RegistryCmd;
use crate::spawn;

use super::AppState;

#[tauri::command]
#[specta::specta]
pub async fn list_repos(state: State<'_, AppState>) -> Result<Vec<RepoEntry>, String> {
    state.config.list_repos().await.map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub async fn add_repo(state: State<'_, AppState>, path: String) -> Result<RepoEntry, String> {
    let path = PathBuf::from(&path);
    spawn::validate_git_repo(&path).map_err(|e| e.to_string())?;
    state.config.add_repo(path).await.map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub async fn remove_repo(state: State<'_, AppState>, name: String) -> Result<(), String> {
    refuse_if_live_sessions(&state, &name).await?;
    state.config.remove_repo(name).await.map_err(Into::into)
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
    state
        .config
        .update_repo_provider(repo_name, provider)
        .await
        .map_err(Into::into)
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
