import { mockCommands } from "../../test/tauri-mock";
import { createViewModeState } from "../view-mode";

describe("view-mode state — mode + diff cache", () => {
  it("defaults missing session to terminal mode", () => {
    const { getMode } = createViewModeState();
    expect(getMode("unknown")).toBe("terminal");
  });

  it("setMode persists per session id", () => {
    const { getMode, setMode } = createViewModeState();
    setMode("s1", "diff");
    expect(getMode("s1")).toBe("diff");
    expect(getMode("s2")).toBe("terminal");
  });

  it("getDiff returns empty entry for unknown session", () => {
    const { getDiff } = createViewModeState();
    expect(getDiff("unknown")).toEqual({ text: "", loading: false, error: null });
  });

  it("clearForSession drops mode and diff state", () => {
    const { setMode, getMode, getDiff, clearForSession } = createViewModeState();
    setMode("s1", "diff");
    clearForSession("s1");
    expect(getMode("s1")).toBe("terminal");
    expect(getDiff("s1")).toEqual({ text: "", loading: false, error: null });
  });
});

describe("view-mode state — refreshDiff IPC", () => {
  it("immediate refresh calls commands.gitDiff and stores text", async () => {
    let calls = 0;
    mockCommands({
      git_diff: (args) => {
        calls++;
        expect(args.worktreePath).toBe("/repo");
        return "diff --git a/x b/x\n+hello\n";
      },
    });
    const { refreshDiff, getDiff } = createViewModeState();
    refreshDiff("s1", "/repo", true);
    await new Promise((r) => setTimeout(r, 0));
    expect(calls).toBe(1);
    expect(getDiff("s1").text).toContain("+hello");
    expect(getDiff("s1").loading).toBe(false);
    expect(getDiff("s1").error).toBeNull();
  });

  it("debounced calls within 500ms collapse to one fetch", async () => {
    vi.useFakeTimers();
    try {
      let calls = 0;
      mockCommands({
        git_diff: () => {
          calls++;
          return "";
        },
      });
      const { refreshDiff } = createViewModeState();
      refreshDiff("s1", "/repo");
      refreshDiff("s1", "/repo");
      refreshDiff("s1", "/repo");
      expect(calls).toBe(0);
      vi.advanceTimersByTime(499);
      expect(calls).toBe(0);
      vi.advanceTimersByTime(2);
      // Let the queued microtask resolve.
      await vi.runAllTimersAsync();
      expect(calls).toBe(1);
    } finally {
      vi.useRealTimers();
    }
  });

  it("surfaces git_diff errors into entry.error", async () => {
    mockCommands({
      git_diff: () => {
        throw "fatal: not a git repository";
      },
    });
    const { refreshDiff, getDiff } = createViewModeState();
    refreshDiff("s1", "/notrepo", true);
    await new Promise((r) => setTimeout(r, 0));
    expect(getDiff("s1").error).toBe("fatal: not a git repository");
    expect(getDiff("s1").loading).toBe(false);
  });
});
