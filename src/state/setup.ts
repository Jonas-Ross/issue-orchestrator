import { signal } from "@preact/signals";
import type { SetupState } from "../lib/bindings";

export const setupState = signal<SetupState | null>(null);
