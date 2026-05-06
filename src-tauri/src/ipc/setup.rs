use tauri::State;

use crate::config::Config;

use super::AppState;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SetupState {
    pub setup_done: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn get_setup_state(state: State<'_, AppState>) -> Result<SetupState, String> {
    let setup_done = state.config.get_setup_state().await.map_err(|e| e.to_string())?;
    Ok(SetupState { setup_done })
}

#[tauri::command]
#[specta::specta]
pub async fn mark_setup_done(state: State<'_, AppState>) -> Result<(), String> {
    state.config.mark_setup_done().await.map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub async fn get_config(state: State<'_, AppState>) -> Result<Config, String> {
    state.config.snapshot().await.map_err(Into::into)
}
