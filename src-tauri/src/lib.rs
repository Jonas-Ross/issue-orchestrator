pub mod config;
pub mod error;
pub mod hooks;
pub mod ipc;
pub mod paths;
pub mod pty;
pub mod registry;
pub mod spawn;

use std::sync::{Arc, Mutex};

use tauri::Manager;
use tauri_specta::{collect_commands, collect_events, Builder};
use tracing::{error, info, warn};

pub use error::{Error, Result};

/// Build the shared tauri-specta `Builder`.
///
/// Single source of truth for the IPC contract: both the runtime app and
/// the `export-bindings` binary call this so `src/lib/bindings.ts` can
/// never drift from the Rust commands and event types.
pub fn make_specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            ipc::pty_spawn,
            ipc::pty_write,
            ipc::pty_resize,
            ipc::pty_kill,
            ipc::list_sessions,
            ipc::get_setup_state,
            ipc::mark_setup_done,
            ipc::get_config,
            ipc::list_repos,
            ipc::list_issues,
            ipc::spawn_issue_session,
        ])
        .events(collect_events![
            ipc::events::PtyData,
            ipc::events::StatusChange,
            ipc::events::SessionAdded,
            ipc::events::SessionRemoved,
        ])
}

pub fn run() {
    init_tracing();
    info!("starting issue-orchestrator");

    let config_path = paths::config_path().expect("compute config path");
    let sock_path = paths::hooks_socket_path().expect("compute hooks socket path");
    let log_path = paths::hooks_log_path().expect("compute hooks log path");
    let hook_script_path = paths::hook_script_path().expect("compute hook script path");

    if let Err(e) = hooks::script::ensure_hook_script(&hook_script_path) {
        warn!(?e, "failed to write hook script");
    } else {
        info!(path = %hook_script_path.display(), "hook script ready");
    }

    let config = config::Config::load_or_default(&config_path).unwrap_or_else(|e| {
        warn!(?e, "config load failed; using defaults");
        config::Config::default()
    });

    let builder = make_specta_builder();

    tauri::Builder::default()
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);

            let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
            let registry_tx = registry::SessionRegistryActor::spawn(event_tx);
            ipc::spawn_event_bridge(app.handle().clone(), event_rx);

            let registry_for_hooks = registry_tx.clone();
            let sock_for_hooks = sock_path.clone();
            let log_for_hooks = log_path.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    hooks::run_listener(sock_for_hooks, log_for_hooks, registry_for_hooks).await
                {
                    error!(?e, "hook listener exited");
                }
            });

            app.manage(ipc::AppState {
                registry: registry_tx,
                config_path: config_path.clone(),
                hook_script_path: hook_script_path.clone(),
                config: Mutex::new(config.clone()),
                issue_client: Arc::new(spawn::GhCli),
                git_runner: Arc::new(spawn::GitCli),
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("issue_orchestrator=info,warn"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}
