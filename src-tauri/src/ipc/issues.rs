use std::path::PathBuf;

use tauri::State;

use crate::issues;
use crate::registry::SessionSummary;
use crate::spawn::{self, Decision, Issue};

use super::{lookup_repo, AppState};

#[tauri::command]
#[specta::specta]
pub async fn list_issues(
    state: State<'_, AppState>,
    repo_name: String,
) -> Result<Vec<Issue>, String> {
    let repo = lookup_repo(&state, &repo_name)?;
    let client = issues::make_client(&repo, &state.http).map_err(|e| e.to_string())?;
    let path = PathBuf::from(&repo.path);
    client.list(&path).await.map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub async fn get_issue_body(
    state: State<'_, AppState>,
    repo_name: String,
    issue_id: String,
) -> Result<String, String> {
    let repo = lookup_repo(&state, &repo_name)?;
    let client = issues::make_client(&repo, &state.http).map_err(|e| e.to_string())?;
    let path = PathBuf::from(&repo.path);
    client.body(&path, &issue_id).await.map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub async fn decide_next_issue(
    state: State<'_, AppState>,
    repo_name: String,
) -> Result<Decision, String> {
    let repo = lookup_repo(&state, &repo_name)?;
    let client = issues::make_client(&repo, &state.http).map_err(|e| e.to_string())?;
    spawn::decide_next_issue(&repo, client)
        .await
        .map_err(Into::into)
}

/// Persist (or clear, when `template = None`) the user-configured spawn
/// prompt template. Atomic-saves the config file via the existing
/// `Config::save` path.
#[tauri::command]
#[specta::specta]
pub async fn update_spawn_prompt(
    state: State<'_, AppState>,
    template: Option<String>,
) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.spawn_prompt_template = template.filter(|t| !t.trim().is_empty());
    config.save(&state.config_path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Ask `claude -p` (running in the chosen repo's cwd) to rewrite the
/// supplied template using whatever skills/MCPs/plugins are visible to a
/// session there. Returns the rewritten template; the caller is
/// responsible for calling `update_spawn_prompt` to actually persist it.
#[tauri::command]
#[specta::specta]
pub async fn optimize_spawn_prompt(
    state: State<'_, AppState>,
    repo_name: String,
    current_prompt: String,
) -> Result<String, String> {
    let repo = lookup_repo(&state, &repo_name)?;
    spawn::optimize_spawn_prompt(&repo, &current_prompt)
        .await
        .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub async fn spawn_issue_session(
    state: State<'_, AppState>,
    repo_name: String,
    issue_id: String,
    cols: u16,
    rows: u16,
    prompt_override: Option<String>,
) -> Result<SessionSummary, String> {
    let repo = lookup_repo(&state, &repo_name)?;
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let client = issues::make_client(&repo, &state.http).map_err(|e| e.to_string())?;

    spawn::spawn_issue_session(
        &repo,
        issue_id,
        &config,
        prompt_override,
        client,
        state.git_runner.clone(),
        state.registry.clone(),
        cols,
        rows,
    )
    .await
    .map_err(Into::into)
}
