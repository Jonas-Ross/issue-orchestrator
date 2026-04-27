import { signal } from "@preact/signals";

export const paletteOpen = signal(false);

export function openPalette() {
  paletteOpen.value = true;
}

export function closePalette() {
  paletteOpen.value = false;
}

export function togglePalette() {
  paletteOpen.value = !paletteOpen.value;
}
