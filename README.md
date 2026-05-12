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
- **First-run setup panel.** Walks you through installing the
  Claude Code plugin and adding your first repo.
- **In-app settings.** ⌘, to add/remove repos and change the
  worktree root.
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

Hooks ship as a Claude Code plugin so there is nothing to hand-paste.
In **any Claude Code session** (a plain `claude` in a terminal works
fine — the orchestrator app doesn't need to be running for the install
itself), run these two slash commands:

```
/plugin marketplace add Jonas-Ross/issue-orchestrator
/plugin install issue-orchestrator@issue-orchestrator
```

Then restart Claude Code (or run `/reload-plugins`). The plugin
registers the four hook events (`SessionStart`, `Notification`,
`Stop`, `SessionEnd`) the orchestrator listens for, pointed at a
script Claude Code installs under its own plugin directory.

When the app is running the plugin forwards events to its Unix
socket and the per-session status pill flips through running /
needs-input / idle. When the app is **not** running the plugin's
script silently exits, so other Claude Code sessions are unaffected.

On first launch the app writes:

```
~/Library/Application Support/app.issue-orchestrator.desktop/
├── config.json     # repos, worktree root, setup flag
├── hooks.sock      # UDS, recreated on each app start
└── events.jsonl    # Append-only audit log of every hook event
```

### Migrating from the manual snippet

If you used a previous build that asked you to paste a hooks block
into `~/.claude/settings.json`, install the plugin as above, then
**delete the orchestrator hooks block** from `~/.claude/settings.json`
— the one whose `command` references
`Library/Application Support/<bundle-id>/hook.sh`. Restart Claude
Code. Until you remove it, that stale block will keep emitting
"Failed with non-blocking status code" errors because its hardcoded
script path no longer exists.

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
  "version": 2,
  "worktreeRoot": "~/dev/worktrees",
  "repos": [
    {
      "name": "issue-team",
      "path": "/Users/you/Documents/GitHub/issue-team",
      "provider": { "kind": "github" }
    }
  ],
  "spawnPromptTemplate": null,
  "setupDone": true
}
```

Per-repo `provider` can be `{"kind": "github"}`, `{"kind": "jira",
"baseUrl": "...", "email": "...", "projectKey": "..."}`, or `{"kind":
"linear", "teamKey": "..."}`. Jira/Linear tokens live in the macOS
Keychain (Settings → Repos → Set token), never in this file.

`spawnPromptTemplate` is optional; `null` falls back to the built-in
default (`"Use the issue-team skill to implement issue #{issue_id}
({issue_title})."`).

Edit through the in-app Settings panel rather than the file directly —
the app does atomic save (write-to-temp + rename) and the panel
validates paths.

---

## Architecture

See [CLAUDE.md](CLAUDE.md) for the full tour. The short version:

- **Actor-owned registry** (`src-tauri/src/registry/`) is the single
  owner of all session state. Mutations go through a `tokio::mpsc`
  mailbox; no shared `Mutex<HashMap>`. The same pattern wraps the
  on-disk `Config` (`src-tauri/src/config/`).
- **Typed IPC contract** via `specta` + `tauri-specta`. The Rust
  command/event surface generates `src/lib/bindings.ts` (gitignored,
  regenerated on every dev launch). Add a command? `cargo tauri dev`
  catches drift at TS compile time.
- **Hook bridge** (`src-tauri/src/hooks/` ← listener;
  `plugins/issue-orchestrator/scripts/hook.sh` ← script shipped to
  Claude Code as a plugin): Claude Code → plugin's `hook.sh` →
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
├── registry/              # SessionRegistryActor, RegistryCmd / Event,
│                          # PTY command builder + env filter
├── ipc/                   # #[tauri::command] surface split per domain:
│                          # pty / setup / repos / issues / secrets / git
├── hooks/                 # UDS listener + JSONL audit log
│                          # (the script itself ships under plugins/)
├── spawn/                 # spawn_issue_session, GitRunner trait,
│                          # headless `claude -p` drivers
├── issues/                # IssueClient trait + GitHub / Jira / Linear
│                          # impls + Keychain-backed secrets
├── config/                # Config types + ConfigActor (mailbox-owned)
├── error.rs               # Typed Error variants per subsystem
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
cargo test --manifest-path src-tauri/Cargo.toml --lib   # backend
npm test                                                 # frontend
```

Backend covers the four pillars:

- **Registry actor** — spawn → kill round-trip, write to unknown
  session, real-PTY `PtyData` event flow.
- **Hook receiver** — status mapping for
  `SessionStart`/`Notification`/`Stop`/`SessionEnd`, audit-log
  persistence, pretty-printed payload tolerance, hooks for unknown
  sessions silently dropped.
- **Spawn flow** — new-branch path uses `git worktree add -b`,
  existing-branch uses bare `worktree add`, existing-worktree skips
  git entirely. Mocks `IssueClient` and `GitRunner` so no real `gh`
  or worktrees are touched.
- **Issue providers** — Jira and Linear HTTP clients tested via
  `wiremock`; auth headers, error paths, GraphQL error handling.

Frontend uses Vitest + jsdom + `@testing-library/preact` with the
Tauri command/event mocks in `src/test/tauri-mock.ts`. Component
coverage is selective (heavy components like `TerminalView` /
`IssuePicker` defer to E2E); state modules and the smaller
components are fully covered. See `CLAUDE.md` → "Test strategy"
for the full breakdown.

### Try it out

After `cargo tauri dev`:

1. Window opens; sidebar empty.
2. ⌘⇧B → a `bash` session appears, prompt within ~1s. Type, ctrl-c,
   resize the window or collapse the sidebar — the terminal reflows
   without garbage.
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
