import { signal } from "@preact/signals";
import type { SetupState } from "../lib/bindings";

export function createSetupStore() {
  const setupState = signal<SetupState | null>(null);
  return { setupState };
}

export const setupStore = createSetupStore();
export const { setupState } = setupStore;
