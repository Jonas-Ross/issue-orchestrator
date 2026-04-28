import { signal } from "@preact/signals";

/// When non-null, the picker is open. `repoName` may prefill the repo so
/// the picker skips its repo-selection step (set by per-drawer "+ new"
/// buttons). Null `repoName` falls back to the legacy single-repo
/// autoselect behavior used by the global ⌘N shortcut.
export interface PickerState {
  repoName: string | null;
}

export const pickerOpen = signal<PickerState | null>(null);

export function openPicker(repoName: string | null = null) {
  pickerOpen.value = { repoName };
}

export function closePicker() {
  pickerOpen.value = null;
}
