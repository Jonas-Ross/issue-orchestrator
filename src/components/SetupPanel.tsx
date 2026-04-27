import { commands } from "../lib/bindings";
import { setupState } from "../state/setup";

/// First-run panel: shows the JSON snippet the user needs to paste into
/// ~/.claude/settings.json so Claude Code's hook system forwards events
/// to the orchestrator. Hidden once `markSetupDone` flips the persisted
/// flag.
export function SetupPanel() {
  const state = setupState.value;
  if (!state || state.setupDone) return null;

  const snippet = buildSnippet(state.hookScriptPath);

  const dismiss = async () => {
    const result = await commands.markSetupDone();
    if (result.status === "error") {
      console.error("markSetupDone failed:", result.error);
      return;
    }
    setupState.value = { ...state, setupDone: true };
  };

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(snippet);
    } catch (e) {
      console.error("clipboard write failed:", e);
    }
  };

  return (
    <div class="setup-overlay">
      <div class="setup-panel">
        <h2>Wire up Claude Code hooks</h2>
        <p>
          Add the following to <code>~/.claude/settings.json</code> under the
          top-level <code>"hooks"</code> key. This lets Claude Code report
          session status back to the orchestrator over a Unix socket.
        </p>
        <pre class="snippet">{snippet}</pre>
        <p class="hint">
          Hook script lives at <code>{state.hookScriptPath}</code>. It will be
          rewritten on every app start, so do not edit it directly.
        </p>
        <div class="setup-actions">
          <button type="button" onClick={() => void copy()}>
            Copy snippet
          </button>
          <button type="button" class="primary" onClick={() => void dismiss()}>
            Done — don't show again
          </button>
        </div>
      </div>
    </div>
  );
}

function buildSnippet(hookScriptPath: string): string {
  // Claude Code pipes `command` through /bin/sh -c, so paths with spaces
  // (notably macOS's "~/Library/Application Support/…") need shell-level
  // quoting or sh splits on the first space and fails to find the bin.
  const cmd = { type: "command", command: shellQuote(hookScriptPath) };
  const entry = [{ hooks: [cmd] }];
  const hooks = {
    SessionStart: entry,
    Notification: entry,
    Stop: entry,
    SessionEnd: entry,
  };
  return JSON.stringify({ hooks }, null, 2);
}

function shellQuote(s: string): string {
  return `'${s.replace(/'/g, "'\\''")}'`;
}
