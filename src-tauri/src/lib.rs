pub mod error;
pub mod ipc;
pub mod pty;
pub mod registry;

use tauri::Manager;
use tauri_specta::{collect_commands, collect_events, Builder};

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
    tracing::info!("starting issue-orchestrator");

    let builder = make_specta_builder();

    tauri::Builder::default()
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);

            let registry_tx = registry::SessionRegistryActor::spawn(app.handle().clone());
            app.manage(ipc::AppState {
                registry: registry_tx,
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
