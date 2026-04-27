use tauri_specta::{collect_commands, collect_events, Builder};

/// Build the shared tauri-specta `Builder`.
///
/// Used by both the runtime app (for `.invoke_handler` / `.mount_events`)
/// and the `export-bindings` binary (for `.export`). Keeping a single
/// source of truth means the TypeScript bindings can never drift from
/// the Rust commands and event types.
///
/// Phase 0: empty. Commands and events are registered as the M2/M3/M4
/// modules land.
pub fn make_specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![])
        .events(collect_events![])
}
