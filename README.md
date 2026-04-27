# issue-orchestrator

Single-window desktop app for running and watching multiple Claude Code
`issue-team` sessions in parallel. Embedded terminals via Tauri 2 + Rust +
xterm.js. Goal: stop juggling tmux panes and Terminal.app windows.

**Status:** Milestone 1 — single PTY round-trip. See `MILESTONES.md` for the roadmap.

## Run it

One-time:

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Tauri CLI
cargo install tauri-cli --version "^2.0" --locked

# Frontend deps
npm install
```

Dev loop:

```bash
cargo tauri dev
```

The first build is slow (Tauri pulls in webkit bindings). Subsequent rebuilds
are seconds.

## What lives where

| Path | Role |
|---|---|
| `src-tauri/src/main.rs` | Tauri builder, app state, command registration |
| `src-tauri/src/pty.rs` | `PtySession`: spawn / read / write / resize |
| `src-tauri/src/commands.rs` | Three `#[tauri::command]` handlers |
| `src-tauri/Cargo.toml` | Three deps: `tauri`, `tauri-build`, `portable-pty` |
| `src-tauri/tauri.conf.json` | Window config, dev URL, identifier |
| `src-tauri/capabilities/default.json` | IPC permissions (Tauri 2 capability) |
| `src/main.ts` | xterm setup + Tauri IPC wiring |
| `index.html` | Single `<div id="terminal">` mount point |
| `MILESTONES.md` | Roadmap M1 → M4 |

## Verify M1

After `cargo tauri dev`:

1. Window opens, terminal area shows a `bash` prompt within ~1s.
2. `ls`, `cd`, env vars work normally.
3. `vim README.md` enters vim, `:q` exits cleanly.
4. `htop` renders; resize the window → terminal reflows without garbage.
5. `sleep 30` → `ctrl-c` interrupts.
6. Cmd-Q → no orphaned `bash` in `ps aux | grep bash`.

If all six pass, M1 is done.

## Reading order for someone learning Rust

Start at `src-tauri/src/main.rs` (tiny — wires everything together), then
`commands.rs` (three small `#[tauri::command]` functions, all the IPC surface),
then `pty.rs` (the only file with real Rust complexity — `Mutex`, threads, and
trait objects). The Tauri-specific patterns live in `main.rs` and the JSON
config; the Rust-specific patterns all live in `pty.rs`.
