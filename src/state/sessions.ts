import { signal, computed } from "@preact/signals";
import type { SessionSummary, Status } from "../lib/bindings";

export const sessions = signal<SessionSummary[]>([]);
export const activeId = signal<string | null>(null);

export const activeSession = computed(
  () => sessions.value.find((s) => s.id === activeId.value) ?? null,
);

/// Sentinel bucket key for sessions without a repo (the Bash debug shell).
export const SHELL_BUCKET = "__shell";

/// Sessions grouped by `repoName`, with `null` repo names collected under
/// `SHELL_BUCKET`. Used by the sidebar to render per-repo drawers.
export const sessionsByRepo = computed<Map<string, SessionSummary[]>>(() => {
  const grouped = new Map<string, SessionSummary[]>();
  for (const s of sessions.value) {
    const key = s.repoName ?? SHELL_BUCKET;
    const bucket = grouped.get(key);
    if (bucket) bucket.push(s);
    else grouped.set(key, [s]);
  }
  return grouped;
});

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
