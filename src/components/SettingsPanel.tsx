import { useEffect, useRef, useState } from "preact/hooks";
import { commands } from "../lib/bindings";
import type { Config } from "../lib/bindings";
import { useFocusRestore, useFocusTrap } from "../lib/use-focus-trap";
import { closeSettings, settings, settingsPanelOpen, updateSetting } from "../state/settings";
import { repos } from "../state/repos";
import { sessions } from "../state/sessions";
import { Modal } from "./Modal";
import { PromptSection } from "./SettingsPanel/PromptSection";

/// Thin wrapper that mounts/unmounts the inner panel around the open
/// signal. Same pattern as IssuePicker — keeps hooks contract clean.
export function SettingsPanel() {
  if (!settingsPanelOpen.value) return null;
  return <SettingsPanelInner />;
}

interface Category {
  id: string;
  label: string;
  glyph: string;
  Render: () => preact.ComponentChildren;
}

const CATEGORIES: Category[] = [
  {
    id: "repos",
    label: "Repos",
    glyph: "⌘",
    Render: () => <ReposSection />,
  },
  {
    id: "prompt",
    label: "Prompt",
    glyph: "✎",
    Render: () => <PromptSection />,
  },
  {
    id: "about",
    label: "About",
    glyph: "ⓘ",
    Render: () => <AboutSection />,
  },
];

function SettingsPanelInner() {
  const modalRef = useRef<HTMLDivElement | null>(null);
  const [activeId, setActiveId] = useState<string>(CATEGORIES[0].id);
  const active = CATEGORIES.find((c) => c.id === activeId) ?? CATEGORIES[0];

  useEffect(() => {
    modalRef.current?.focus();
  }, []);
  useFocusRestore();
  useFocusTrap(modalRef);

  return (
    <Modal
      onClose={() => closeSettings()}
      dialogClass="settings-shell"
      dialogRef={modalRef}
      tabIndex={-1}
    >
      <header class="settings-header">
        <h2>Settings</h2>
        <button type="button" class="close" onClick={() => closeSettings()} title="Close (Esc)">
          ×
        </button>
      </header>

      <div class="settings-body">
        <nav class="settings-nav">
          {CATEGORIES.map((c) => (
            <button
              key={c.id}
              type="button"
              class={`settings-nav-item${c.id === activeId ? " active" : ""}`}
              onClick={() => setActiveId(c.id)}
            >
              <span class="settings-nav-glyph">{c.glyph}</span>
              <span class="settings-nav-label">{c.label}</span>
            </button>
          ))}
        </nav>

        <section class="settings-pane">
          <h3 class="settings-pane-title">{active.label}</h3>
          <div class="settings-pane-body">{active.Render()}</div>
        </section>
      </div>
    </Modal>
  );
}

// ── Categories ─────────────────────────────────────────────────────────

function ReposSection() {
  const current = settings.value;
  return (
    <>
      <ToggleRow
        label="Show paste-path field"
        description="When off, the AddRepoButton only exposes the native folder picker. Turn on for the power-user fallback that accepts arbitrary absolute paths."
        checked={current.pastePathEnabled}
        onChange={(v) => updateSetting("pastePathEnabled", v)}
      />
    </>
  );
}

function AboutSection() {
  const repoCount = repos.value.length;
  const sessionCount = sessions.value.length;
  const [config, setConfig] = useState<Config | null>(null);

  useEffect(() => {
    void (async () => {
      const result = await commands.getConfig();
      if (result.status === "ok") setConfig(result.data);
    })();
  }, []);

  return (
    <dl class="settings-info">
      <InfoRow label="App version" value={APP_VERSION} mono />
      <InfoRow label="Worktree root" value={config?.worktreeRoot ?? "Loading…"} mono />
      <InfoRow label="Configured repos" value={String(repoCount)} />
      <InfoRow label="Active sessions" value={String(sessionCount)} />
    </dl>
  );
}

// Bumped manually for now; if we ever want to wire it to package.json,
// vite's `define` config can inject __APP_VERSION__ at build time.
const APP_VERSION = "0.1.0";

// ── Building blocks ────────────────────────────────────────────────────

interface ToggleRowProps {
  label: string;
  description?: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}

function ToggleRow({ label, description, checked, onChange }: ToggleRowProps) {
  return (
    <label class="settings-row">
      <div class="settings-row-text">
        <span class="settings-row-label">{label}</span>
        {description && <span class="settings-row-desc">{description}</span>}
      </div>
      <button
        type="button"
        class={`settings-toggle${checked ? " on" : ""}`}
        role="switch"
        aria-checked={checked}
        onClick={() => onChange(!checked)}
      >
        <span class="settings-toggle-knob" />
      </button>
    </label>
  );
}

interface InfoRowProps {
  label: string;
  value: string;
  mono?: boolean;
}

function InfoRow({ label, value, mono }: InfoRowProps) {
  return (
    <div class="settings-info-row">
      <dt class="settings-info-label">{label}</dt>
      <dd class={`settings-info-value${mono ? " mono" : ""}`}>{value}</dd>
    </div>
  );
}
