use tauri::{AppHandle, State};

use crate::pty::PtySession;
use crate::AppState;

#[tauri::command]
pub fn pty_spawn(
    app: AppHandle,
    state: State<'_, AppState>,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let mut slot = state.session.lock().map_err(|e| e.to_string())?;
    if slot.is_some() {
        // Idempotent: M1 only ever has one PTY. Re-mounting the frontend
        // (e.g. on dev hot-reload) shouldn't spawn a second shell.
        return Ok(());
    }
    *slot = Some(PtySession::spawn(app, cols, rows)?);
    Ok(())
}

#[tauri::command]
pub fn pty_write(state: State<'_, AppState>, data: String) -> Result<(), String> {
    let slot = state.session.lock().map_err(|e| e.to_string())?;
    let session = slot.as_ref().ok_or("pty not spawned")?;
    session.write(&data)
}

#[tauri::command]
pub fn pty_resize(state: State<'_, AppState>, cols: u16, rows: u16) -> Result<(), String> {
    let slot = state.session.lock().map_err(|e| e.to_string())?;
    let session = slot.as_ref().ok_or("pty not spawned")?;
    session.resize(cols, rows)
}
