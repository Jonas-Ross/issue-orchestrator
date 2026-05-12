use std::path::PathBuf;

use tauri::State;

use super::AppState;

#[tauri::command]
#[specta::specta]
pub async fn git_diff(
    state: State<'_, AppState>,
    worktree_path: String,
) -> Result<String, String> {
    let path = PathBuf::from(&worktree_path);
    state.git_runner.diff(&path).map_err(|e| e.to_string())
}
