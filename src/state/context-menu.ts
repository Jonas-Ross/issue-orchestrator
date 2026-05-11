import { signal } from "@preact/signals";

export type ContextMenuItem =
  | { label: string; action: () => void; disabled?: boolean }
  | { separator: true };

/**
 * Which corner of the menu the `(x, y)` coordinate refers to. Default
 * `top-left` keeps the existing right-click behaviour (menu opens
 * down-and-to-the-right of the cursor). `bottom-right` lets a button
 * pinned to the right edge of a narrow sidebar drop its menu
 * up-and-to-the-left instead of pushing into the terminal area.
 */
export type ContextMenuAnchor = "top-left" | "bottom-right";

export interface ContextMenuState {
  x: number;
  y: number;
  items: ContextMenuItem[];
  anchor?: ContextMenuAnchor;
}

export function createContextMenuState() {
  const contextMenu = signal<ContextMenuState | null>(null);
  const openContextMenu = (state: ContextMenuState) => {
    contextMenu.value = state;
  };
  const closeContextMenu = () => {
    contextMenu.value = null;
  };
  return { contextMenu, openContextMenu, closeContextMenu };
}

export const contextMenuStore = createContextMenuState();
export const { contextMenu, openContextMenu, closeContextMenu } = contextMenuStore;
