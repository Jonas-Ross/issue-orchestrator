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
pub fn get_setup_state(state: State<'_, AppState>) -> Result<SetupState, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(SetupState {
        setup_done: config.setup_done,
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
