# issue-orchestrator

Single-window desktop app for running and watching multiple Claude Code
`issue-team` sessions in parallel. Embedded terminals via Tauri 2 + Rust +
xterm.js + Preact. Goal: stop juggling tmux panes and Terminal.app windows.

**Status:** M1–M4 landed. The app spawns N orchestrated `claude` sessions
in their own git worktrees, surfaces per-tab status driven by Claude Code's
hook system, and fully decouples its core from any Tauri-specific state so
the registry, hook receiver, and spawn flow are all unit-tested.

## Run it

One-time:

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Tauri CLI
cargo install tauri-cli --version "^2.0" --locked

# Frontend deps
npm install

# A few CLIs the app shells out to:
brew install gh jq
gh auth login
```

Dev loop:

```bash
cargo tauri dev
```

The first build is slow (Tauri pulls in webkit bindings + the new dep tree
including specta and tokio process). Subsequent rebuilds are seconds.

## Configure

On first launch the app writes:

```
~/Library/Application Support/dev.jonasross.issue-orchestrator/
├── config.json     # repos, worktree root, setup flag
├── hook.sh         # Generated; do not edit
├── hooks.sock      # UDS, recreated on each app start
└── events.jsonl    # Append-only audit log of every hook event
```

Edit `config.json` to add the repos you want to spawn issues from:

```jsonc
{
  "version": 1,
  "worktreeRoot": "~/dev/worktrees",
  "repos": [
    { "name": "issue-team", "path": "/Users/you/Documents/GitHub/issue-team" }
  ],
  "setupDone": false
}
```

Then restart the app. The `+` button in the tab strip opens the issue
picker; click an issue and the app:

1. `gh issue view` to grab the title
2. `git worktree add` either reusing `issue-<n>` or creating a fresh
   branch under `<worktreeRoot>/<repoName>-issue-<n>`
3. spawns `claude` in that worktree with `ISSUE_ORCH_SESSION_ID=<uuid>`
   and the prompt `"Use the issue-team skill to implement issue #<n>."`

The first time the app runs, a setup panel shows the JSON to paste into
`~/.claude/settings.json` so Claude Code's hooks forward
SessionStart/Notification/Stop/SessionEnd events to the app's UDS. Without
that wiring the per-tab status pill stays gray; with it the pill flips
green/amber/blue as Claude works.

The snippet single-quotes the script path because macOS's
`~/Library/Application Support/` contains a space and Claude Code pipes
the command through `/bin/sh -c`. If you hand-edit the entry, keep the
quotes.

## What lives where

| Path | Role |
|---|---|
| `src-tauri/src/main.rs` | Thin entry — calls `lib::run()` |
| `src-tauri/src/lib.rs` | Tauri builder, registers commands and event bridge |
| `src-tauri/src/pty.rs` | Stateless `spawn_pty` + UTF-8 leftover-byte buffer |
| `src-tauri/src/registry/` | `SessionRegistryActor`, `RegistryCmd`, `RegistryEvent` |
| `src-tauri/src/ipc/` | `#[tauri::command]` surface + Tauri Specta event types |
| `src-tauri/src/hooks/` | Tokio UDS listener, JSONL log, hook.sh generator |
| `src-tauri/src/spawn.rs` | `IssueClient` / `GitRunner` traits + `spawn_issue_session` |
| `src-tauri/src/config.rs` | JSON config load/save with atomic rename |
| `src-tauri/src/paths.rs` | App-data path helpers |
| `src-tauri/src/bin/export-bindings.rs` | Regenerates `src/lib/bindings.ts` |
| `src/main.tsx` / `src/app.tsx` | Preact mount + top-level layout |
| `src/components/` | TabStrip, Tab, TerminalArea, TerminalView, IssuePicker, SetupPanel |
| `src/state/` | Signals: sessions, picker, setup; pty-stream demuxer |
| `src/lib/bindings.ts` | Generated; gitignored |

## Verify M1

After `cargo tauri dev`:

1. Window opens; tab strip is empty with `+` and `⌘` buttons.
2. Click `⌘` → a `bash` tab appears, prompt within ~1s. Type, ctrl-c, etc.
3. Open a second `bash` tab; switch between → both keep scrollback.
4. Resize window in either tab → reflows without garbage.
5. Click × on a tab → PTY killed, no orphaned `bash` in `ps aux`.
6. Cmd-Q → all sessions cleaned up.

## Verify M3 + M4

1. Edit `config.json` to point at a real repo (any repo with `gh auth`
   set up will do).
2. Click `+` → issue picker shows your repo's open issues.
3. Pick one → tab opens, `claude` boots in
   `~/dev/worktrees/<repo>-issue-<n>` with the issue-team skill prompted.
4. As Claude runs, the tab's status pill goes green (Running), amber
   (NeedsInput when Claude pauses for input), and blue (Idle on Stop).
5. `tail -f ~/Library/Application\ Support/dev.jonasross.issue-orchestrator/events.jsonl`
   to watch the raw hook stream.

## Tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Covers:
- Registry actor: spawn → kill round-trip, write to unknown session,
  PtyData event flow with a real bash PTY.
- Hook receiver: status mapping for Notification/Stop, audit log
  persistence, hooks for unknown sessions ignored.
- Spawn flow: new-branch path uses `git worktree add -b`, existing-branch
  uses bare `worktree add`, existing-worktree skips git entirely. Mocks
  `IssueClient` and `GitRunner` so no `gh` or real worktrees are touched.
