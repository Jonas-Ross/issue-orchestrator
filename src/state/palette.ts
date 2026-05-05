import { signal } from "@preact/signals";

export function createPaletteState() {
  const paletteOpen = signal(false);
  const openPalette = () => {
    paletteOpen.value = true;
  };
  const closePalette = () => {
    paletteOpen.value = false;
  };
  const togglePalette = () => {
    paletteOpen.value = !paletteOpen.value;
  };
  return { paletteOpen, openPalette, closePalette, togglePalette };
}

export const paletteState = createPaletteState();
export const { paletteOpen, openPalette, closePalette, togglePalette } =
  paletteState;
