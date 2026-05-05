import { signal, computed } from "@preact/signals";
import type { SessionSummary, Status } from "../lib/bindings";

/// Sentinel bucket key for sessions without a repo (the Bash debug shell).
export const SHELL_BUCKET = "__shell";

export function createSessionsState() {
  const sessions = signal<SessionSummary[]>([]);
  const activeId = signal<string | null>(null);

  const activeSession = computed(
    () => sessions.value.find((s) => s.id === activeId.value) ?? null,
  );

  /// Sessions grouped by `repoName`, with `null` repo names collected under
  /// `SHELL_BUCKET`. Used by the sidebar to render per-repo drawers.
  const sessionsByRepo = computed<Map<string, SessionSummary[]>>(() => {
    const grouped = new Map<string, SessionSummary[]>();
    for (const s of sessions.value) {
      const key = s.repoName ?? SHELL_BUCKET;
      const bucket = grouped.get(key);
      if (bucket) bucket.push(s);
      else grouped.set(key, [s]);
    }
    return grouped;
  });

  function addSession(s: SessionSummary) {
    if (sessions.value.some((x) => x.id === s.id)) return;
    sessions.value = [...sessions.value, s];
    if (activeId.value === null) activeId.value = s.id;
  }

  function removeSession(id: string) {
    sessions.value = sessions.value.filter((s) => s.id !== id);
    if (activeId.value === id) {
      activeId.value = sessions.value[0]?.id ?? null;
    }
  }

  function setStatus(id: string, status: Status) {
    sessions.value = sessions.value.map((s) =>
      s.id === id ? { ...s, status } : s,
    );
  }

  return {
    sessions,
    activeId,
    activeSession,
    sessionsByRepo,
    addSession,
    removeSession,
    setStatus,
  };
}

export const sessionsStore = createSessionsState();
export const {
  sessions,
  activeId,
  activeSession,
  sessionsByRepo,
  addSession,
  removeSession,
  setStatus,
} = sessionsStore;
