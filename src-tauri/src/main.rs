// Hide the extra console window on Windows release builds. Harmless on macOS.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod pty;

use std::sync::Mutex;

use tracing_subscriber::EnvFilter;

use crate::pty::PtySession;

/// Tauri-managed shared state. Held in a Mutex because Tauri commands run
/// concurrently and any of them may touch the active session.
#[derive(Default)]
pub struct AppState {
    pub session: Mutex<Option<PtySession>>,
}

fn main() {
    init_tracing();

    tracing::info!("starting issue-orchestrator");

    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::pty_spawn,
            commands::pty_write,
            commands::pty_resize,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("issue_orchestrator=info,warn"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
