# issue-orchestrator — M2 → M4 build plan

## Context

Right now the app is a single-PTY scaffold (`MILESTONES.md` M1): one bash session in one Tauri window. The goal of this build is to land milestones M2 (multi-session registry), M3 (hook receiver), and M4 (spawn flow) so the app actually fulfills its purpose: **launching and watching N Claude Code `issue-team` sessions in parallel**, each in its own git worktree, with live status updates from Claude's hook system surfaced in the UI.

The user is prioritizing **structural quality and maintainability** over speed. This plan therefore introduces real module boundaries, a typed IPC contract between Rust and TypeScript, an actor-based concurrency model on the backend, and a small reactive frontend stack — choices that pay back as the codebase grows past M4 and as more contributors touch it.

---

## Locked architecture decisions

| Concern | Choice | Why |
|---|---|---|
| Frontend stack | **Preact + `@preact/signals`** | ~8KB, fine-grained reactivity, JSX, zero hooks ceremony |
| IPC type contract | **`specta` + `tauri-specta`** | Generates a fully typed TS client from Rust structs; drift caught at compile time |
| Backend concurrency | **`SessionRegistryActor` + `tokio::mpsc`** | Single owner of the session HashMap; no lock-ordering bugs; trivially testable |
| Hook → session correlation | **Env-var injected session ID** (`ISSUE_ORCH_SESSION_ID`) | Robust to user `cd`, unambiguous, doesn't depend on cwd state |

### Supporting choices (decided, not surfaced — flag any you'd change)

- **Async runtime**: Tauri's bundled tokio runtime. Hook listener uses `tauri::async_runtime::spawn`. PTY reader stays on `std::thread` (blocking I/O).
- **Logging**: `tracing` + `tracing-subscriber` (env-filter, pretty in dev, JSON in release).
- **Errors**: `thiserror` enum per module (`pty::Error`, `registry::Error`, `hooks::Error`, `spawn::Error`); commands return `Result<T, String>` for IPC ergonomics, mapping via a single `into_ipc` helper.
- **Config**: JSON at `<app-data>/config.json`. Specta-typed so the schema is shared with the frontend.
- **Worktree layout**: `<worktree_root>/<repo_name>-issue-<n>`. `worktree_root` is configurable, defaults to `~/dev/worktrees`.
- **No session persistence across app restart** (sessions die with the app; the JSONL hook log is audit-only). Persistence is a future milestone — call out if you want it in scope.
- **Hook script delivery**: app writes `<app-data>/hook.sh` on first launch and shows a copy-paste snippet for `~/.claude/settings.json` in a one-time Setup panel. We do **not** auto-edit the user's settings.

---

## Module layout

### Backend — `src-tauri/src/`

```
main.rs            Tauri builder; registers commands; spawns registry actor + hook listener
lib.rs             Re-exports for tests
error.rs           Top-level Error enum + Result alias
paths.rs           App-data dir helpers (wraps Tauri's path::app_data_dir)
config.rs          Config struct, load/save, default values
pty.rs             Low-level PTY: spawn_pty(cmd, env, cols, rows) -> PtyHandles
                   No shared state. Reader thread takes a mpsc::Sender<PtyEvent>.
registry/
  mod.rs           SessionRegistryActor, RegistryCmd enum, Session struct
  session.rs       Session: id, status, pty_handles, worktree_path, claude_session_id
  status.rs        Status enum + transitions
hooks/
  mod.rs           HookListener (tokio UnixListener), HookEvent types
  log.rs           JSONL append-only audit log writer
spawn.rs           spawn_issue_session: gh issue view → worktree add → claude launch
                   Boundary trait IssueClient so gh is mockable
ipc/
  mod.rs           #[tauri::command] handlers (specta-annotated); thin wrappers around actor
  events.rs        Specta-typed event payloads (PtyData, StatusChange, SessionAdded, …)
  bindings.rs      `bin/export_bindings.rs` writes ../src/lib/bindings.ts
```

### Frontend — `src/`

```
main.tsx                       Preact root mount
app.tsx                        Layout: <TabStrip> + <TerminalArea> + <ModalHost>
state/
  sessions.ts                  signals: sessions, activeId; derived: activeSession
  issues.ts                    issue picker state
  config.ts                    config signal (loaded once on boot)
components/
  TabStrip.tsx
  Tab.tsx                      title + status pill + × close button
  TerminalArea.tsx             host div; mounts one <TerminalView> per session, hides inactive
  TerminalView.tsx             wraps xterm; one instance per session; lifecycle bound to signal
  IssuePicker.tsx              modal: repo dropdown → issue list
  SetupPanel.tsx               first-run hook-config snippet
lib/
  ipc.ts                       re-exports specta-generated commands + events
  bindings.ts                  GENERATED, gitignored
styles/
  app.css, tabs.css, modal.css
```

---

## Build sequence

### Phase 0 — Foundation refactor

Land this *before* M2 so every subsequent change benefits.

1. **Deps**
   - Cargo: `uuid` (v4), `tokio` (rt-multi-thread, net, sync, io-util, macros), `serde` (derive), `serde_json`, `specta`, `specta-typescript`, `tauri-specta`, `thiserror`, `tracing`, `tracing-subscriber` (env-filter), `dirs`.
   - Tauri: enable `tauri-specta` integration in `tauri.conf.json` if needed.
   - npm: `preact`, `@preact/signals`, `@preact/preset-vite` (dev). Update `tsconfig.json`: `"jsx": "react-jsx"`, `"jsxImportSource": "preact"`. Update `vite.config.ts` to include `preact()`.
2. **Tracing setup** in `main.rs` (env-filter, default `info`).
3. **Specta export binary** at `src-tauri/src/bin/export_bindings.rs` writes to `../src/lib/bindings.ts`. Add `predev` and `prebuild` npm scripts to invoke it.
4. **Refactor entrypoint**: rename `src/main.ts` → `src/main.tsx`; render `<App />` (empty for now); update `index.html` script tag. Verify `cargo tauri dev` still boots and renders the existing single bash session (we'll move xterm into `TerminalView` in Phase 1).

### Phase 1 — M2: registry actor + multi-session UI

1. **`pty.rs`** (state-free): `pub fn spawn_pty(cmd: CommandBuilder, size: PtySize) -> Result<PtyHandles>`. `PtyHandles { writer, master, child, reader_thread_join }`. Reader thread takes a `mpsc::Sender<PtyEvent>` (`PtyEvent::Data(Vec<u8>) | PtyEvent::Eof`). **UTF-8 fix**: maintain a leftover-bytes buffer in the reader so multi-byte sequences split across reads aren't mangled.
2. **`registry/`**:
   - `RegistryCmd` enum: `Spawn { … , reply: oneshot::Sender<SessionId> }`, `Write { id, data }`, `Resize { id, cols, rows }`, `Kill { id }`, `List { reply }`, `HookEvent(HookEvent)`.
   - `SessionRegistryActor::run(rx, app_handle)` owns `HashMap<SessionId, Session>`. For each spawn, calls `pty::spawn_pty`, stashes `PtyHandles`, spawns a forwarder task that reads `PtyEvent`s and emits `pty:data` Tauri events with payload `PtyData { session_id, chunk }` (specta-typed).
3. **Commands** (`ipc/mod.rs`): `pty_spawn { command_kind: SpawnKind } -> SessionId`, `pty_write { id, data }`, `pty_resize { id, cols, rows }`, `pty_kill { id }`, `list_sessions() -> Vec<SessionSummary>`. Each is a thin wrapper that sends a `RegistryCmd` and (where applicable) awaits the oneshot reply.
4. **Frontend**:
   - `sessions: Signal<Map<SessionId, SessionView>>`. `SessionView { id, title, status, term: xterm.Terminal, fitAddon }`.
   - `<TabStrip>`: maps over `sessions.value`, highlights `activeId`, "+" opens picker (in M2: just spawns a bash session for now).
   - `<TerminalArea>`: renders one `<TerminalView session={s}>` per session inside a host div; non-active ones get `display: none`. On activate, calls `fit.fit()` and `pty_resize`.
   - `<TerminalView>`: on mount, attaches xterm to its div, wires `term.onData → pty_write`. Listens to `pty:data` once at module level and routes by `session_id`.
5. **Tests**: Rust unit test that spawns the actor, sends `RegistryCmd::Spawn` with a mock `spawn_pty` returning fake handles, asserts session appears in `List`, then `Kill` removes it.

### Phase 2 — M3: hook receiver

1. **`hooks/mod.rs`**: `HookListener::start(tx: mpsc::Sender<RegistryCmd>, sock_path: PathBuf) -> JoinHandle`. Tokio `UnixListener`; per connection, read newline-delimited JSON; parse into `HookEvent { hook_event_name, session_orch_id: Option<String>, claude_session_id, cwd, transcript_path }`; for each event call `log::append(&event)` then `tx.send(RegistryCmd::HookEvent(event)).await`.
2. **Status mapping** (in actor's `HookEvent` arm): match `session_orch_id` → session; transition `Status` per `hook_event_name`:
   - `SessionStart` → `Running` (also stash `claude_session_id`, `transcript_path`)
   - `Notification` → `NeedsInput`
   - `Stop` → `Idle`
   - `SessionEnd` → `Exited`
   Then emit `status:change` event with `StatusChange { session_id, status }`.
3. **Hook script** generation (`hooks/script.rs`): on app start, idempotently write `<app-data>/hook.sh` with a header comment containing a version stamp; chmod +x. Script:
   ```bash
   #!/bin/bash
   sock="$HOME/Library/Application Support/dev.jonasross.issue-orchestrator/hooks.sock"
   [ -S "$sock" ] || exit 0
   payload=$(cat)
   echo "$payload" | jq --arg orch_id "${ISSUE_ORCH_SESSION_ID:-}" \
     '. + ($orch_id | select(length > 0) | {session_orch_id: .} // {})' \
     | nc -U "$sock" -w 1 2>/dev/null || true
   ```
4. **`<SetupPanel>`** (first-run): shows the JSON to paste into `~/.claude/settings.json`:
   ```json
   "hooks": {
     "SessionStart":  [{"hooks":[{"type":"command","command":"<app-data>/hook.sh"}]}],
     "Notification":  [{"hooks":[{"type":"command","command":"<app-data>/hook.sh"}]}],
     "Stop":          [{"hooks":[{"type":"command","command":"<app-data>/hook.sh"}]}],
     "SessionEnd":    [{"hooks":[{"type":"command","command":"<app-data>/hook.sh"}]}]
   }
   ```
   Dismissible; `setup_done: true` written to config.
5. **`<Tab>`** renders a colored pill driven by `Status`: gray=Spawning, green=Running, amber=NeedsInput, blue=Idle, red=Exited.
6. **Tests**: integration test that starts `HookListener` on a temp socket, sends a fake `Notification` JSON, asserts the actor receives `RegistryCmd::HookEvent` and the session's status is `NeedsInput`.

### Phase 3 — M4: spawn flow

1. **`config.rs`** schema:
   ```jsonc
   {
     "version": 1,
     "worktree_root": "~/dev/worktrees",
     "repos": [
       { "name": "issue-team", "path": "/Users/jonasross/Documents/GitHub/issue-team" }
     ],
     "setup_done": false
   }
   ```
   `~` expansion via `shellexpand`. Atomic save (write-to-temp-then-rename). Specta-derive so the frontend reads the same type.
2. **`spawn.rs`**:
   - `trait IssueClient { fn list(&self, repo: &Path) -> Result<Vec<Issue>>; fn view(&self, repo: &Path, n: u64) -> Result<Issue>; }` with a `GhCli` impl that shells out to `gh`.
   - `pub async fn spawn_issue_session(repo: &RepoEntry, issue_n: u64, registry_tx: …) -> Result<SessionId>`:
     1. `orch_id = Uuid::new_v4()`
     2. `wt_path = config.worktree_root / format!("{}-issue-{}", repo.name, issue_n)`
     3. Branch handling:
        - If `git -C <repo.path> rev-parse --verify issue-<n>` succeeds → existing branch: `git worktree add <wt_path> issue-<n>`
        - Else → new branch: `git worktree add <wt_path> -b issue-<n>`
        - If `<wt_path>` already exists *and* is a registered worktree → reuse (no add)
     4. Build `CommandBuilder::new("claude")`; cwd = `wt_path`; env `ISSUE_ORCH_SESSION_ID=<orch_id>`; arg = `"Use the issue-team skill to implement issue #<n>."`
     5. Send `RegistryCmd::Spawn { command, orch_id, worktree_path, … }` to actor; await session id.
3. **Commands**:
   - `list_repos() -> Vec<RepoEntry>`
   - `list_issues(repo_name) -> Vec<Issue>`  (uses `IssueClient`)
   - `spawn_issue_session(repo_name, issue_number) -> SessionId`
4. **Frontend**:
   - `<IssuePicker>`: opened by "+". Repo dropdown (auto-skip if 1 repo). Issue list with title + labels + number. Click → `spawn_issue_session` → tab opens & is activated. Loading + error states per fetch.
5. **Tests**:
   - Unit test for `spawn_issue_session` with a mock `IssueClient` and a mock git runner (extract a tiny `GitRunner` trait too) — assert the right git args are issued and a `RegistryCmd::Spawn` is sent with the right cwd + env.

---

## Verification

End-to-end manual test once all three phases land:

1. `cargo tauri dev` → window opens, no terminal yet.
2. Click "+" → `<IssuePicker>` modal lists open issues from the configured repo.
3. Pick an issue → tab appears, status pill goes gray (Spawning) → green (Running) within ~3s.
4. Inside the new tab: `claude` is running in `~/dev/worktrees/<repo>-issue-<n>`, with the `issue-team` skill prompted.
5. Open another issue in a second tab → both run in parallel; switching tabs preserves scrollback in each.
6. When Claude in tab 1 emits a `Notification` hook → tab 1's pill flips amber.
7. When Claude in tab 1 finishes and emits `Stop` → tab 1's pill flips blue.
8. Close tab 2 via × → PTY killed, no orphaned `claude` in `ps aux`.
9. Cmd-Q → all sessions cleaned up; `<app-data>/events.jsonl` contains the audit trail.

Programmatic checks:

- `cargo test -p issue-orchestrator` passes (registry actor + hook listener integration + spawn flow unit tests).
- `cargo run --bin export-bindings` regenerates `src/lib/bindings.ts` cleanly (no diff on a clean build).
- `npm run build` succeeds (TS compile validates the IPC contract).

Type-drift smoke test:

- Add a field to `HookEvent` in Rust → `cargo run --bin export-bindings` → `tsc` fails until the frontend handles it. Confirms the contract is enforced.

---

## Critical files (touch / create)

**Modified**:
- `src-tauri/Cargo.toml` (deps)
- `src-tauri/src/main.rs` (actor + listener wiring, tracing init)
- `src-tauri/src/pty.rs` (state-free refactor, UTF-8 fix)
- `src-tauri/tauri.conf.json` (specta integration if needed)
- `src-tauri/capabilities/default.json` (no change expected; verify)
- `src/main.ts` → `src/main.tsx`
- `index.html` (script tag)
- `package.json` (deps + predev/prebuild scripts)
- `tsconfig.json` (jsx: react-jsx, jsxImportSource: preact)
- `vite.config.ts` (preact preset)
- `MILESTONES.md` (mark M2/M3/M4 done as we land them)

**Created**:
- `src-tauri/src/lib.rs`, `error.rs`, `paths.rs`, `config.rs`, `spawn.rs`
- `src-tauri/src/registry/{mod,session,status}.rs`
- `src-tauri/src/hooks/{mod,log,script}.rs`
- `src-tauri/src/ipc/{mod,events}.rs`
- `src-tauri/src/bin/export_bindings.rs`
- `src/app.tsx`
- `src/state/{sessions,issues,config}.ts`
- `src/components/{TabStrip,Tab,TerminalArea,TerminalView,IssuePicker,SetupPanel}.tsx`
- `src/lib/ipc.ts`
- `src/styles/{app,tabs,modal}.css`
- `src/lib/bindings.ts` (generated, gitignored)

---

## Assumptions

1. **macOS only** — already in MILESTONES; no Windows/Linux conditional code.
2. **Required CLIs on PATH**: `gh` (verified 2.91.0), `git` (verified 2.50.1), `claude` (verified 2.1.119), `nc`, `jq`. App should fail loudly with a useful message if any are missing.
3. **User edits `~/.claude/settings.json` once** to wire the hook script. App generates the script and shows the snippet; does not modify settings.json itself.
4. **No session persistence across app restart** — sessions are ephemeral; quitting kills all PTYs. JSONL audit log persists for debugging only. Flag this if you want persistence in scope; it's a substantial add.
5. **Hooks fired by Claude sessions NOT spawned through the orchestrator are dropped** — no `ISSUE_ORCH_SESSION_ID` env var, no correlation. Honors the strict "env-var only" choice from the architecture decisions; if you want a cwd fallback later, that's a small follow-up in the actor.
6. **`gh` is already authenticated** for the configured repo. We don't handle auth flow.
7. **Worktree branch naming**: `issue-<n>`. If the branch already exists we reuse it (no `-b`); if the worktree dir already exists we reuse it (no `git worktree add`). We never delete worktrees automatically — the user may have unpushed work.
8. **Single window** — no multi-window orchestration.
9. **First `cargo tauri dev` after Phase 0 deps land will be slow** (a few minutes). Subsequent builds are seconds.
10. **The `issue-team` skill is installed and resolvable from any cwd** (verified at `~/.claude/settings.json` `enabledPlugins`).
