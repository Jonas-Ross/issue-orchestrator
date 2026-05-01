import { useState } from "preact/hooks";
import { commands } from "../lib/bindings";
import { copyToClipboard } from "../lib/clipboard";
import { setupState } from "../state/setup";
import { repos } from "../state/repos";
import { AddRepoButton } from "./AddRepoButton";

/// First-run onboarding panel: walks the user through (1) installing
/// the Claude Code plugin and (2) adding their first repo. Hidden once
/// `markSetupDone` flips the persisted flag — it never replays even if
/// the user later removes all repos.
export function SetupPanel() {
  const state = setupState.value;
  const [step, setStep] = useState<1 | 2>(1);
  const repoCount = repos.value.length;

  if (!state || state.setupDone) return null;

  const finish = async () => {
    const result = await commands.markSetupDone();
    if (result.status === "error") {
      console.error("markSetupDone failed:", result.error);
      return;
    }
    setupState.value = { setupDone: true };
  };

  return (
    <div class="setup-overlay">
      <div class="setup-panel">
        <div class="setup-stepper">
          <span class={`setup-step${step === 1 ? " active" : ""}`}>1. Plugin</span>
          <span class="setup-step-sep">·</span>
          <span class={`setup-step${step === 2 ? " active" : ""}`}>2. Repo</span>
        </div>

        {step === 1 && <Step1Plugin onContinue={() => setStep(2)} />}

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

const MARKETPLACE_CMD = "/plugin marketplace add Jonas-Ross/issue-orchestrator";
const INSTALL_CMD = "/plugin install issue-orchestrator@issue-orchestrator";

function Step1Plugin({ onContinue }: { onContinue: () => void }) {
  return (
    <>
      <h2>Install the issue-orchestrator plugin</h2>
      <p>
        Run these in any Claude Code session — they wire Claude's hooks
        into the orchestrator over a Unix socket.
      </p>
      <CommandRow command={MARKETPLACE_CMD} />
      <CommandRow command={INSTALL_CMD} />
      <p class="hint">
        Then restart Claude Code (or run <code>/reload-plugins</code>) for it
        to take effect. The plugin script silently no-ops when this app isn't
        running, so other Claude sessions are unaffected.
      </p>
      <div class="setup-actions">
        <button type="button" class="primary" onClick={onContinue}>
          Continue →
        </button>
      </div>
    </>
  );
}

function CommandRow({ command }: { command: string }) {
  const [status, setStatus] = useState<"idle" | "copied" | "failed">("idle");

  const copy = async () => {
    setStatus((await copyToClipboard(command)) ? "copied" : "failed");
    setTimeout(() => setStatus("idle"), 1500);
  };

  const label =
    status === "copied" ? "Copied" : status === "failed" ? "Select manually" : "Copy";

  return (
    <div class="snippet-row">
      <pre class="snippet">{command}</pre>
      <button type="button" onClick={() => void copy()}>
        {label}
      </button>
    </div>
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
