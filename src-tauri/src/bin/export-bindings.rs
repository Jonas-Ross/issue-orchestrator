use std::path::Path;

use issue_orchestrator::make_specta_builder;
use specta_typescript::Typescript;

/// Standalone binary that regenerates `src/lib/bindings.ts` without
/// booting the full Tauri app. Wired into `npm run predev` and
/// `npm run prebuild` so the frontend always sees the latest IPC
/// contract before vite or tsc runs.
fn main() {
    // CARGO_MANIFEST_DIR points at `src-tauri`; the bindings live one
    // level up under `src/lib/`.
    let out = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir must have a parent")
        .join("src")
        .join("lib")
        .join("bindings.ts");

    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).expect("create bindings parent dir");
    }

    make_specta_builder()
        .export(Typescript::default(), &out)
        .expect("export TypeScript bindings");

    eprintln!("wrote {}", out.display());
}
