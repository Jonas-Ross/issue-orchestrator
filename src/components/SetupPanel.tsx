import { useState } from "preact/hooks";
import { commands } from "../lib/bindings";
import { setupState } from "../state/setup";
import { repos } from "../state/repos";
import { AddRepoButton } from "./AddRepoButton";

/// First-run onboarding panel: walks the user through (1) installing
/// the Claude Code hooks snippet and (2) adding their first repo. Hidden
/// once `markSetupDone` flips the persisted flag — it never replays even
/// if the user later removes all repos.
export function SetupPanel() {
  const state = setupState.value;
  const [step, setStep] = useState<1 | 2>(1);
  const repoCount = repos.value.length;

  if (!state || state.setupDone) return null;

  const snippet = buildSnippet(state.hookScriptPath);

  const finish = async () => {
    const result = await commands.markSetupDone();
    if (result.status === "error") {
      console.error("markSetupDone failed:", result.error);
      return;
    }
    setupState.value = { ...state, setupDone: true };
  };

  return (
    <div class="setup-overlay">
      <div class="setup-panel">
        <div class="setup-stepper">
          <span class={`setup-step${step === 1 ? " active" : ""}`}>1. Hooks</span>
          <span class="setup-step-sep">·</span>
          <span class={`setup-step${step === 2 ? " active" : ""}`}>2. Repo</span>
        </div>

        {step === 1 && (
          <Step1Hooks
            snippet={snippet}
            hookScriptPath={state.hookScriptPath}
            onContinue={() => setStep(2)}
          />
        )}

        {step === 2 && (
          <Step2Repo
            repoCount={repoCount}
            onBack={() => setStep(1)}
            onContinue={() => void finish()}
          />
        )}
      </div>
    </div>
  );
}

function Step1Hooks({
  snippet,
  hookScriptPath,
  onContinue,
}: {
  snippet: string;
  hookScriptPath: string;
  onContinue: () => void;
}) {
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(snippet);
    } catch (e) {
      console.error("clipboard write failed:", e);
    }
  };

  return (
    <>
      <h2>Wire up Claude Code hooks</h2>
      <p>
        Add the following to <code>~/.claude/settings.json</code> under the
        top-level <code>"hooks"</code> key. This lets Claude Code report
        session status back to the orchestrator over a Unix socket.
      </p>
      <pre class="snippet">{snippet}</pre>
      <p class="hint">
        Hook script lives at <code>{hookScriptPath}</code>. It will be
        rewritten on every app start, so do not edit it directly.
      </p>
      <div class="setup-actions">
        <button type="button" onClick={() => void copy()}>
          Copy snippet
        </button>
        <button type="button" class="primary" onClick={onContinue}>
          Continue →
        </button>
      </div>
    </>
  );
}

function Step2Repo({
  repoCount,
  onBack,
  onContinue,
}: {
  repoCount: number;
  onBack: () => void;
  onContinue: () => void;
}) {
  return (
    <>
      <h2>Add your first repo</h2>
      <p>
        Pick the folder of a Git repository you want to drive issue-team
        sessions in. You can add more later from the sidebar.
      </p>
      <AddRepoButton variant="primary" />
      <div class="setup-actions">
        <button type="button" onClick={onBack}>
          ← Back
        </button>
        <button
          type="button"
          class="primary"
          disabled={repoCount === 0}
          onClick={onContinue}
          title={repoCount === 0 ? "Add a repo first" : "Finish setup"}
        >
          Done
        </button>
      </div>
    </>
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
