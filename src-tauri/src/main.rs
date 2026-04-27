// Hide the extra console window on Windows release builds. Harmless on macOS.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    issue_orchestrator::run();
}
