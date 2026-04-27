import { signal } from "@preact/signals";

export const pickerOpen = signal(false);

export function openPicker() {
  pickerOpen.value = true;
}

export function closePicker() {
  pickerOpen.value = false;
}
