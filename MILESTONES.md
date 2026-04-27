# Milestones

## M1 — PTY round-trip *(this scaffold)*

**Goal:** `bash` running inside the app window with reflow, ctrl-c, and
clean exit on quit.

**Files of interest:**
- `src-tauri/src/pty.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/main.rs`
- `src/main.ts`

**Acceptance:** see README "Verify M1".

**Things this scaffold already handles (so you don't trip over them):**
- Reader runs on `std::thread::spawn` — `portable-pty`'s `read` is blocking,
  so a real OS thread is correct here, not a tokio task.
- `Mutex` around master (resize) and writer (write) because Tauri commands
  may run concurrently.
- `cmd.env("TERM", "xterm-256color")` so colored TUIs render correctly.
- The parent's env is copied into the PTY explicitly — `CommandBuilder`
  starts with an empty env in portable-pty 0.8.
- `drop(pair.slave)` after spawn so the child owns the slave fd.
- `Drop` impl on `PtySession` calls `child.kill()` so quitting the app
  doesn't leave orphaned shells.

---

## M2 — Multi-session registry

**Goal:** N PTYs running side-by-side, tab strip in the UI, kill flow.

**Backend changes:**
- Replace `Mutex<Option<PtySession>>` in `AppState` with
  `Mutex<HashMap<String, PtySession>>`.
- Each session gets a UUID at spawn time.
- Reader thread emits to `pty:data:<session_id>` instead of `pty:data`.
- New commands: `pty_kill(session_id)`, `pty_list()`.

**Frontend changes:**
- Tab strip component over the terminal area.
- One `Terminal` instance per session, mounted/unmounted as tabs switch.
- Listen on per-session event names.

**Gotchas:**
- xterm `Terminal` instances are heavy — keep them mounted off-screen rather
  than recreating on tab switch, or you'll lose scrollback.
- Call `term.dispose()` on session kill, not just unmount.
- Add `uuid = "1"` and (probably) `tokio` to deps when this milestone lands.

---

## M3 — Hook receiver

**Goal:** Claude Code sessions report state to the app over a Unix socket.
Status pill per tab: running / needs-input / idle / pr-open.

**New deps:**
- `tokio = { version = "1", features = ["rt-multi-thread", "net", "io-util", "macros"] }`
- `serde = { version = "1", features = ["derive"] }`
- `serde_json = "1"`

**New module:** `src-tauri/src/hooks.rs` — `UnixListener` on
`~/Library/Application Support/issue-orchestrator/hooks.sock`. Accepts
newline-delimited JSON events and dispatches into the session registry.
Append every event to a per-session JSONL log on disk so events fired while
the app is closed get replayed on next startup.

**Hook scripts** (one-time, in `~/.claude/settings.json`):
- `SessionStart` → `nc -U <sock>` posting `{event, session_id, cwd, transcript_path}`.
- `Notification` → same socket, marks session `needs_input`.
- `Stop` → same socket, marks session `idle`.

**Gotchas:**
- Hook payload schema fields to pin: `session_id`, `cwd`, `transcript_path`,
  `hook_event_name`. Lock these in a Rust struct.
- If the app isn't running when hooks fire, the JSONL log on disk is the
  recovery mechanism — replay on startup before the UI shows.
- Don't try to launch the app from the hook script — too fragile. Just write
  to the socket if it exists, fall through to the JSONL log if not.

---

## M4 — Spawn flow

**Goal:** pick an issue from a list, app does worktree creation + claude launch.

**New command:** `spawn_session(issue_number)` →
1. `gh issue view <n> --json title,body,labels`
2. Pick a worktree path (e.g. `~/dev/worktrees/issue-<n>`).
3. `git worktree add <path> -b issue-<n>` from the parent repo dir.
4. Spawn `claude` in that worktree with the `issue-team` skill primed.

**Frontend:** "+" button in the tab strip → modal listing
`gh issue list --json number,title,labels --limit 50`. Click → spawn.

**Reference:** mirror the manual launch flow in
`/Users/jonasross/Documents/GitHub/issue-team/skills/`.

**Gotchas:**
- The parent repo dir for `git worktree add` has to come from somewhere —
  per-app config file with a list of "watched repos".
- Spawned `claude` needs the worktree as cwd, not the parent.
- The new session's `SessionStart` hook self-registers via the socket,
  keying off `cwd` to link it back to the issue number set by the spawn flow.

---

## Out of scope (all milestones)

- Codesigning / notarization / packaging for distribution
- Cross-machine orchestration
- Embedded `gh pr` views (link out instead)
- Persisted layout across restarts
- Windows / Linux support (macOS-only for now)
