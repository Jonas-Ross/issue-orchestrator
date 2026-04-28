import { useEffect, useRef, useState } from "preact/hooks";
import { commands } from "../lib/bindings";
import type { Config } from "../lib/bindings";
import {
  closeSettings,
  settings,
  settingsPanelOpen,
  updateSetting,
} from "../state/settings";
import { repos } from "../state/repos";
import { sessions } from "../state/sessions";
import { setupState } from "../state/setup";

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
    const previous = document.activeElement as HTMLElement | null;
    modalRef.current?.focus();
    return () => {
      previous?.focus?.();
    };
  }, []);

  // Focus trap so Tab can't escape into the sidebar behind the overlay.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Tab") return;
      const root = modalRef.current;
      if (!root) return;
      const focusables = Array.from(
        root.querySelectorAll<HTMLElement>(
          'button:not([disabled]), input:not([disabled]):not([type="hidden"]), select:not([disabled]), [tabindex]:not([tabindex="-1"])',
        ),
      );
      if (focusables.length === 0) {
        e.preventDefault();
        root.focus();
        return;
      }
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      const current = document.activeElement as HTMLElement | null;
      const insideModal = current && root.contains(current);
      if (!insideModal) {
        e.preventDefault();
        (e.shiftKey ? last : first).focus();
        return;
      }
      if (e.shiftKey && current === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && current === last) {
        e.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, []);

  return (
    <div class="modal-overlay" onClick={() => closeSettings()}>
      <div
        class="settings-shell"
        ref={modalRef}
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
      >
        <header class="settings-header">
          <h2>Settings</h2>
          <button
            type="button"
            class="close"
            onClick={() => closeSettings()}
            title="Close (Esc)"
          >
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
      </div>
    </div>
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
  const setup = setupState.value;
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
      <InfoRow
        label="Worktree root"
        value={config?.worktreeRoot ?? "Loading…"}
        mono
      />
      <InfoRow
        label="Hook script"
        value={setup?.hookScriptPath ?? "Not loaded"}
        mono
      />
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
        {description && (
          <span class="settings-row-desc">{description}</span>
        )}
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
