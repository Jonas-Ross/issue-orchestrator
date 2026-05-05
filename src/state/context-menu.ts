import { signal } from "@preact/signals";

export type ContextMenuItem =
  | { label: string; action: () => void; disabled?: boolean }
  | { separator: true };

export interface ContextMenuState {
  x: number;
  y: number;
  items: ContextMenuItem[];
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
export const { contextMenu, openContextMenu, closeContextMenu } =
  contextMenuStore;
