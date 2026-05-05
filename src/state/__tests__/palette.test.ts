import { createPaletteState } from "../palette";

describe("palette state", () => {
  it("starts closed", () => {
    const { paletteOpen } = createPaletteState();
    expect(paletteOpen.value).toBe(false);
  });

  it("openPalette sets true; closePalette sets false", () => {
    const { paletteOpen, openPalette, closePalette } = createPaletteState();
    openPalette();
    expect(paletteOpen.value).toBe(true);
    closePalette();
    expect(paletteOpen.value).toBe(false);
  });

  it("togglePalette flips the value", () => {
    const { paletteOpen, togglePalette } = createPaletteState();
    togglePalette();
    expect(paletteOpen.value).toBe(true);
    togglePalette();
    expect(paletteOpen.value).toBe(false);
  });
});
