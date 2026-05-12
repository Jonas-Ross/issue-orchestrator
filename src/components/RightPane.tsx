import { getMode, setMode, refreshDiff } from "../state/view-mode";
import { TerminalView } from "./TerminalView";
import { DiffView } from "./DiffView";

interface Props {
  sessionId: string;
  active: boolean;
  worktreePath: string | null;
}

/// Right-pane shell: tab strip + body. The TerminalView stays mounted
/// across mode toggles so xterm scrollback survives — when Diff is
/// active we hide the terminal host and overlay DiffView. Sessions
/// without a worktree (Bash debug shell) render only the Terminal tab.
///
/// getMode reads `modes.value.get(id)` — enough for Preact signals to
/// register a subscription so this component re-renders when the mode
/// flips for this session.
export function RightPane({ sessionId, active, worktreePath }: Props) {
  const mode = worktreePath === null ? "terminal" : getMode(sessionId);

  function onRefresh() {
    if (worktreePath === null) return;
    refreshDiff(sessionId, worktreePath, true);
  }

  return (
    <div className="right-pane" style={{ display: active ? "flex" : "none" }}>
      <div className="right-pane-tabs" role="tablist">
        <button
          type="button"
          role="tab"
          aria-selected={mode === "terminal"}
          className="right-pane-tab"
          onClick={() => setMode(sessionId, "terminal")}
        >
          Terminal
        </button>
        {worktreePath !== null && (
          <button
            type="button"
            role="tab"
            aria-selected={mode === "diff"}
            className="right-pane-tab"
            onClick={() => setMode(sessionId, "diff")}
          >
            Diff
          </button>
        )}
        {mode === "diff" && worktreePath !== null && (
          <button
            type="button"
            className="right-pane-tab-refresh"
            aria-label="Refresh diff"
            onClick={onRefresh}
          >
            ↻
          </button>
        )}
      </div>
      <div className="right-pane-body">
        <div
          className="right-pane-terminal"
          style={{ display: mode === "terminal" ? "block" : "none" }}
        >
          <TerminalView sessionId={sessionId} active={active && mode === "terminal"} />
        </div>
        {mode === "diff" && worktreePath !== null && (
          <DiffView sessionId={sessionId} worktreePath={worktreePath} />
        )}
      </div>
    </div>
  );
}
