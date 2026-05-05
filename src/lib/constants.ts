/// Default PTY dimensions for newly-spawned sessions. The terminal
/// resizes to fit the actual container after mount, so these only
/// control the initial buffer the shell sees before the first resize.
/// Mirrors the 80x24 used by the Rust-side test fixtures.
export const DEFAULT_PTY_COLS = 80;
export const DEFAULT_PTY_ROWS = 24;
