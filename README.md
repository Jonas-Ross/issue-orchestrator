# issue-orchestrator

A single-window desktop app for running and watching many Claude Code
`issue-team` sessions in parallel — each in its own `git worktree`,
each with a live status pill driven by Claude's hook system.

Stop juggling tmux panes and Terminal.app windows.

> **Status:** working day-to-day. Tauri 2 + Rust + Preact + xterm.js,
> macOS only. The registry, hook receiver, and spawn flow are all
> unit-tested.

---

## What you get

- **N sessions, one window.** Each issue picks an open `claude` session
  in its own worktree; switch between them in the sidebar.
- **Live status per session.** Hook-driven pills:
  green = running, mint = needs input, blue = idle, red = exited.
- **Native notifications when Claude needs you.** Background sessions
  that flip to *needs input* fire a macOS notification and bounce the
  dock icon — so you can let three sessions run, get pulled back only
  for the one that's blocked.
- **Issue picker with multi-repo support.** ⌘N opens a fuzzy-searchable
  list of open issues across every configured repo.
- **Command palette.** ⌘K for quick switching, killing, spawning
  without leaving the keyboard.
- **First-run setup panel.** Walks you through pasting the hook
  snippet into `~/.claude/settings.json` — no manual digging.
- **In-app settings.** ⌘, to add/remove repos, change the worktree
  root, re-open the setup snippet.
- **Clean shutdown.** Cmd-Q kills every PTY; no orphaned `claude` or
  `bash` left behind.

---

## Install

One-time prereqs:

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Tauri CLI
cargo install tauri-cli --version "^2.0" --locked

# Frontend deps
npm install

# CLIs the app shells out to
brew install gh jq
gh auth login
```

Dev loop:

```bash
cargo tauri dev
```

The first build is slow (Tauri pulls in webkit bindings + the Rust dep
tree); subsequent rebuilds are seconds.

---

## First-run setup

On first launch the app writes:

```
~/Library/Application Support/app.issue-orchestrator.desktop/
├── config.json     # repos, worktree root, setup flag
├── hook.sh         # Generated; do not edit (rewritten on each app start)
├── hooks.sock      # UDS, recreated on each app start
└── events.jsonl    # Append-only audit log of every hook event
```

A **Setup panel** appears with the JSON to paste into
`~/.claude/settings.json` so Claude Code's hooks forward
`SessionStart` / `Notification` / `Stop` / `SessionEnd` events to the
app's UDS.

Without that wiring the per-session status pill stays gray. With it,
the pill flips through running / needs-input / idle as Claude works.

> The snippet single-quotes the script path because macOS's
> `~/Library/Application Support/` contains a space and Claude Code
> pipes the command through `/bin/sh -c`. If you hand-edit the entry,
> keep the quotes.

You can re-open the setup snippet any time from **Settings → About**.

---

## Daily use

1. **Add a repo.** Open Settings (⌘,) → Repos → "Add repo", point it
   at any local clone you have `gh auth` set up for.
2. **Open an issue.** ⌘N → pick a repo → click an issue. The app:
   - `gh issue view` to grab the title
   - `git worktree add` (reusing `issue-<n>` if it exists, else
     creating a fresh branch under `<worktreeRoot>/<repoName>-issue-<n>`)
   - spawns `claude` in that worktree with
     `ISSUE_ORCH_SESSION_ID=<uuid>` and the prompt
     `"Use the issue-team skill to implement issue #<n>."`
3. **Watch.** As Claude runs, the sidebar pill cycles through the
   four states. If a session goes mint (*needs input*) while you're
   in another session, you'll get a desktop notification.
4. **Switch.** ⌘1…⌘9 to jump to a session, ⌘[ / ⌘] to step.
5. **Close.** ⌘W closes the active session (kills the PTY; the
   worktree and branch stay).

---

## Keyboard shortcuts

| Shortcut | Action |
|---|---|
| ⌘N | Open issue picker |
| ⌘K | Open command palette |
| ⌘, | Open settings |
| ⌘B | Toggle sidebar |
| ⌘W | Kill active session |
| ⌘1 … ⌘9 | Switch to session N |
| ⌘[ / ⌘] | Previous / next session |
| ⌘⇧B | Spawn a debug bash session |
| Esc | Close any open modal / palette / menu |

All bindings require `metaKey`, so plain typing in the terminal is
never intercepted.

---

## Configuration

`~/Library/Application Support/app.issue-orchestrator.desktop/config.json`:

```jsonc
{
  "version": 1,
  "worktreeRoot": "~/dev/worktrees",
  "repos": [
    { "name": "issue-team", "path": "/Users/you/Documents/GitHub/issue-team" }
  ],
  "setupDone": true
}
```

Edit through the in-app Settings panel rather than the file directly —
the app does atomic save (write-to-temp + rename) and the panel
validates paths.

---

## Architecture

See [CLAUDE.md](CLAUDE.md) for the full tour. The short version:

- **Actor-owned registry** (`src-tauri/src/registry/`) is the single
  owner of all session state. Mutations go through a `tokio::mpsc`
  mailbox; no shared `Mutex<HashMap>`.
- **Typed IPC contract** via `specta` + `tauri-specta`. The Rust
  command/event surface generates `src/lib/bindings.ts` (gitignored,
  regenerated on every dev launch). Add a command? `cargo tauri dev`
  catches drift at TS compile time.
- **Hook bridge** (`src-tauri/src/hooks/`): Claude Code → `hook.sh` →
  `nc -U hooks.sock` → tokio `UnixListener` → `RegistryCmd::HookEvent`
  → status pill updates in the UI. Correlation key is the
  `ISSUE_ORCH_SESSION_ID` env var, not cwd, so users can `cd` freely.
- **PTY layer** (`src-tauri/src/pty.rs`): stateless `spawn_pty`,
  blocking reader on `std::thread`, UTF-8 leftover-byte buffer so
  multibyte glyphs split across reads aren't mangled.

```
src-tauri/src/
├── main.rs / lib.rs       # Tauri builder, command + event registration
├── pty.rs                 # Stateless PTY spawn
├── registry/              # SessionRegistryActor + RegistryCmd / Event
├── ipc/                   # #[tauri::command] surface, specta event types
├── hooks/                 # UDS listener, JSONL log, hook.sh generator
├── spawn/                 # IssueClient + GitRunner traits, spawn_issue_session
├── config.rs              # JSON config load/save with atomic rename
├── paths.rs               # App-data path helpers (macOS hardcoded)
└── bin/export-bindings.rs # Regenerates src/lib/bindings.ts

src/
├── main.tsx / app.tsx     # Preact mount + top-level layout
├── components/            # Sidebar, TerminalView, IssuePicker, CommandPalette,
│                          # SettingsPanel, SetupPanel, StatusBar, ContextMenu, …
├── state/                 # Signals: sessions, repos, picker, palette, settings,
│                          # setup, sidebar, notifications, keymap, pty-stream
└── lib/bindings.ts        # GENERATED, gitignored
```

---

## Tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Covers the three pillars:

- **Registry actor** — spawn → kill round-trip, write to unknown
  session, real-PTY `PtyData` event flow.
- **Hook receiver** — status mapping for
  `SessionStart`/`Notification`/`Stop`/`SessionEnd`, audit-log
  persistence, hooks for unknown sessions silently dropped.
- **Spawn flow** — new-branch path uses `git worktree add -b`,
  existing-branch uses bare `worktree add`, existing-worktree skips
  git entirely. Mocks `IssueClient` and `GitRunner` so no real `gh`
  or worktrees are touched.

There are no frontend tests yet; UI verification is manual.

### Try it out

After `cargo tauri dev`:

1. Window opens; sidebar empty.
2. ⌘⇧B → a `bash` session appears, prompt within ~1s. Type, ctrl-c,
   resize the window — the terminal reflows without garbage.
3. ⌘⇧B again → second session; ⌘1 / ⌘2 toggles, both keep scrollback.
4. ⌘W → PTY killed, no orphaned `bash` in `ps aux`.
5. Configure a repo (Settings → Repos), then ⌘N → pick an issue. A
   `claude` session boots in
   `~/dev/worktrees/<repo>-issue-<n>` with the issue-team skill
   prompted. Pill goes green within a few seconds.
6. `tail -f ~/Library/Application\ Support/app.issue-orchestrator.desktop/events.jsonl`
   to watch the raw hook stream.
7. Cmd-Q → all sessions cleaned up.

---

## Out of scope (deliberately)

Don't open PRs adding any of these without discussion first:

- Codesigning / notarization / packaging for distribution
- Windows or Linux support
- Cross-machine orchestration
- Embedded `gh pr` views (the app links out to GitHub instead)
- Session persistence across app restart
