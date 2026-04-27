import { signal, computed } from "@preact/signals";
import type { SessionSummary, Status } from "../lib/bindings";

export const sessions = signal<SessionSummary[]>([]);
export const activeId = signal<string | null>(null);

export const activeSession = computed(
  () => sessions.value.find((s) => s.id === activeId.value) ?? null,
);

export function addSession(s: SessionSummary) {
  if (sessions.value.some((x) => x.id === s.id)) return;
  sessions.value = [...sessions.value, s];
  if (activeId.value === null) activeId.value = s.id;
}

export function removeSession(id: string) {
  sessions.value = sessions.value.filter((s) => s.id !== id);
  if (activeId.value === id) {
    activeId.value = sessions.value[0]?.id ?? null;
  }
}

export function setStatus(id: string, status: Status) {
  sessions.value = sessions.value.map((s) =>
    s.id === id ? { ...s, status } : s,
  );
}
