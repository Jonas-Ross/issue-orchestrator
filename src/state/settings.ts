import { signal, effect } from "@preact/signals";

const STORAGE_KEY = "io.settings.v1";

export interface Settings {
  /// Show the "Paste path" affordance in the AddRepoButton. When false,
  /// only the native folder picker is exposed. Off by default — power
  /// users can flip it back on from the settings panel.
  pastePathEnabled: boolean;
}

const DEFAULT_SETTINGS: Settings = {
  pastePathEnabled: false,
};

function loadSettings(): Settings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT_SETTINGS;
    const parsed = JSON.parse(raw);
    return { ...DEFAULT_SETTINGS, ...parsed };
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export const settings = signal<Settings>(loadSettings());

effect(() => {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings.value));
  } catch {
    // best effort; storage may be disabled
  }
});

export function updateSetting<K extends keyof Settings>(key: K, value: Settings[K]) {
  settings.value = { ...settings.value, [key]: value };
}

export const settingsPanelOpen = signal(false);

export function openSettings() {
  settingsPanelOpen.value = true;
}

export function closeSettings() {
  settingsPanelOpen.value = false;
}
