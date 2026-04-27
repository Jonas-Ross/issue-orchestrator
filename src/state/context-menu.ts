import { signal } from "@preact/signals";

export type ContextMenuItem =
  | { label: string; action: () => void; disabled?: boolean }
  | { separator: true };

export interface ContextMenuState {
  x: number;
  y: number;
  items: ContextMenuItem[];
}

export const contextMenu = signal<ContextMenuState | null>(null);

export function openContextMenu(state: ContextMenuState) {
  contextMenu.value = state;
}

export function closeContextMenu() {
  contextMenu.value = null;
}
