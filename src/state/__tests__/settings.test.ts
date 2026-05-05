import { createSettingsStore, STORAGE_KEY } from "../settings";

describe("settings store", () => {
  it("loads defaults when storage is empty", () => {
    const { settings } = createSettingsStore();
    expect(settings.value).toEqual({ pastePathEnabled: false });
  });

  it("loads persisted values, merging with defaults", () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ pastePathEnabled: true }));
    const { settings } = createSettingsStore();
    expect(settings.value.pastePathEnabled).toBe(true);
  });

  it("recovers from corrupt JSON in storage", () => {
    localStorage.setItem(STORAGE_KEY, "{not json");
    const { settings } = createSettingsStore();
    expect(settings.value).toEqual({ pastePathEnabled: false });
  });

  it("ignores unknown keys but keeps known ones", () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ pastePathEnabled: true, futureKey: "x" }),
    );
    const { settings } = createSettingsStore();
    expect(settings.value.pastePathEnabled).toBe(true);
    expect((settings.value as unknown as Record<string, unknown>).futureKey).toBe("x");
  });

  it("updateSetting persists to storage immediately", () => {
    const { updateSetting } = createSettingsStore();
    updateSetting("pastePathEnabled", true);
    const persisted = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null");
    expect(persisted.pastePathEnabled).toBe(true);
  });

  it("openSettings/closeSettings flip the panel signal", () => {
    const { settingsPanelOpen, openSettings, closeSettings } =
      createSettingsStore();
    expect(settingsPanelOpen.value).toBe(false);
    openSettings();
    expect(settingsPanelOpen.value).toBe(true);
    closeSettings();
    expect(settingsPanelOpen.value).toBe(false);
  });

  it("isolated stores don't share state via signals", () => {
    const a = createSettingsStore();
    const b = createSettingsStore();
    a.updateSetting("pastePathEnabled", true);
    expect(a.settings.value.pastePathEnabled).toBe(true);
    // b has its own signal but reads from the same shared storage; the
    // factory only loads once at construction, so b's view is stale.
    expect(b.settings.value.pastePathEnabled).toBe(false);
  });
});
