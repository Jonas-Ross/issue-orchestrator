import { signal } from "@preact/signals";
import { commands, events } from "../lib/bindings";
import { sessions } from "./sessions";

export type ViewMode = "terminal" | "diff";

export interface DiffEntry {
  text: string;
  loading: boolean;
  error: string | null;
}

const EMPTY_DIFF: DiffEntry = { text: "", loading: false, error: null };
const DEBOUNCE_MS = 500;

/// Per-session right-pane mode + diff cache. The mode default is
/// `terminal`; sessions absent from the map fall through to that. Diff
/// text is `git diff HEAD --no-color` from the session's worktree —
/// tracked + staged changes only, no untracked files.
export function createViewModeState() {
  const modes = signal<Map<string, ViewMode>>(new Map());
  const diffs = signal<Map<string, DiffEntry>>(new Map());
  const debounceTimers = new Map<string, ReturnType<typeof setTimeout>>();
  let started = false;

  function getMode(id: string): ViewMode {
    return modes.value.get(id) ?? "terminal";
  }

  function setMode(id: string, mode: ViewMode) {
    const next = new Map(modes.value);
    next.set(id, mode);
    modes.value = next;
  }

  function getDiff(id: string): DiffEntry {
    return diffs.value.get(id) ?? EMPTY_DIFF;
  }

  function updateDiff(id: string, patch: Partial<DiffEntry>) {
    const prev = diffs.value.get(id) ?? EMPTY_DIFF;
    const next = new Map(diffs.value);
    next.set(id, { ...prev, ...patch });
    diffs.value = next;
  }

  async function fetchDiff(id: string, worktreePath: string) {
    updateDiff(id, { loading: true, error: null });
    const result = await commands.gitDiff(worktreePath);
    if (result.status === "ok") {
      updateDiff(id, { text: result.data, loading: false, error: null });
    } else {
      updateDiff(id, { loading: false, error: result.error });
    }
  }

  /// Trigger a `git diff` fetch for this session.
  ///
  /// `immediate` skips the debounce — used by tab-click and the manual
  /// refresh button. Default (debounced) collapses bursts from rapid
  /// status-change events into one trailing-edge fetch 500ms after the
  /// last call.
  function refreshDiff(id: string, worktreePath: string, immediate = false) {
    const existing = debounceTimers.get(id);
    if (existing !== undefined) clearTimeout(existing);
    if (immediate) {
      debounceTimers.delete(id);
      void fetchDiff(id, worktreePath);
      return;
    }
    const handle = setTimeout(() => {
      debounceTimers.delete(id);
      void fetchDiff(id, worktreePath);
    }, DEBOUNCE_MS);
    debounceTimers.set(id, handle);
  }

  function clearForSession(id: string) {
    const t = debounceTimers.get(id);
    if (t !== undefined) {
      clearTimeout(t);
      debounceTimers.delete(id);
    }
    if (modes.value.has(id)) {
      const nextModes = new Map(modes.value);
      nextModes.delete(id);
      modes.value = nextModes;
    }
    if (diffs.value.has(id)) {
      const nextDiffs = new Map(diffs.value);
      nextDiffs.delete(id);
      diffs.value = nextDiffs;
    }
  }

  /// Subscribe to status + session-removed events so Diff tabs
  /// auto-refresh when Claude reports activity, and view-mode entries
  /// don't outlive their session. Called once by app.tsx; tests
  /// construct fresh state via the factory and skip this.
  function startViewMode(): void {
    if (started) return;
    started = true;
    void events.statusChange.listen((e) => {
      const id = e.payload.sessionId;
      if (getMode(id) !== "diff") return;
      const session = sessions.value.find((s) => s.id === id);
      if (!session?.worktreePath) return;
      refreshDiff(id, session.worktreePath);
    });
    void events.sessionRemoved.listen((e) => {
      clearForSession(e.payload.sessionId);
    });
  }

  return {
    modes,
    diffs,
    getMode,
    setMode,
    getDiff,
    refreshDiff,
    clearForSession,
    startViewMode,
  };
}

export const viewModeStore = createViewModeState();
export const {
  modes,
  diffs,
  getMode,
  setMode,
  getDiff,
  refreshDiff,
  clearForSession,
  startViewMode,
} = viewModeStore;
