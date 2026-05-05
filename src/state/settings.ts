import { signal, effect } from "@preact/signals";

export const STORAGE_KEY = "io.settings.v1";

export interface Settings {
  /// Show the "Paste path" affordance in the AddRepoButton. When false,
  /// only the native folder picker is exposed. Off by default — power
  /// users can flip it back on from the settings panel.
  pastePathEnabled: boolean;
}

const DEFAULT_SETTINGS: Settings = {
  pastePathEnabled: false,
};

function loadSettings(storage: Storage): Settings {
  try {
    const raw = storage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT_SETTINGS;
    const parsed = JSON.parse(raw);
    return { ...DEFAULT_SETTINGS, ...parsed };
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export function createSettingsStore(storage: Storage = localStorage) {
  const settings = signal<Settings>(loadSettings(storage));

  effect(() => {
    try {
      storage.setItem(STORAGE_KEY, JSON.stringify(settings.value));
    } catch {
      // best effort; storage may be disabled
    }
  });

  function updateSetting<K extends keyof Settings>(key: K, value: Settings[K]) {
    settings.value = { ...settings.value, [key]: value };
  }

  const settingsPanelOpen = signal(false);
  const openSettings = () => {
    settingsPanelOpen.value = true;
  };
  const closeSettings = () => {
    settingsPanelOpen.value = false;
  };

  return {
    settings,
    settingsPanelOpen,
    updateSetting,
    openSettings,
    closeSettings,
  };
}

export const settingsStore = createSettingsStore();
export const { settings, settingsPanelOpen, updateSetting, openSettings, closeSettings } =
  settingsStore;
