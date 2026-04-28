use std::path::PathBuf;

use crate::error::{Error, Result};

const APP_IDENTIFIER: &str = "app.issue-orchestrator.desktop";

/// Compute `~/Library/Application Support/<identifier>` and ensure it
/// exists. Hardcoded to the macOS layout — Tauri's own path API derives
/// the same value from the bundle identifier.
pub fn app_data_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| Error::Config("HOME not set".into()))?;
    let dir = home
        .join("Library")
        .join("Application Support")
        .join(APP_IDENTIFIER);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn config_path() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("config.json"))
}

pub fn hooks_socket_path() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("hooks.sock"))
}

pub fn hooks_log_path() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("events.jsonl"))
}

pub fn hook_script_path() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("hook.sh"))
}
