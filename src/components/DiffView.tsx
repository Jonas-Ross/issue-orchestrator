import { useEffect } from "preact/hooks";
import { getDiff, refreshDiff } from "../state/view-mode";

interface Props {
  sessionId: string;
  worktreePath: string;
}

type LineKind = "meta" | "hunk" | "add" | "del" | "context";

function classify(line: string): LineKind {
  if (line.startsWith("diff --git ") || line.startsWith("index ")) return "meta";
  if (line.startsWith("--- ") || line.startsWith("+++ ")) return "meta";
  if (line.startsWith("@@")) return "hunk";
  if (line.startsWith("+")) return "add";
  if (line.startsWith("-")) return "del";
  return "context";
}

export function DiffView({ sessionId, worktreePath }: Props) {
  // getDiff reads `diffs.value.get(id)` — enough for Preact signals to
  // register a subscription so this component re-renders when the diff
  // cache is updated by refreshDiff.
  const entry = getDiff(sessionId);

  useEffect(() => {
    refreshDiff(sessionId, worktreePath, true);
  }, [sessionId, worktreePath]);

  if (entry.error !== null) {
    return (
      <div className="diff-view diff-view-error" role="alert">
        <div className="diff-error-message">{entry.error}</div>
        <button
          type="button"
          className="diff-retry"
          onClick={() => refreshDiff(sessionId, worktreePath, true)}
        >
          Retry
        </button>
      </div>
    );
  }

  if (entry.loading && entry.text === "") {
    return <div className="diff-view diff-view-loading">Loading…</div>;
  }

  if (entry.text === "") {
    return <div className="diff-view diff-view-empty">Working tree clean</div>;
  }

  const lines = entry.text.split("\n");
  return (
    <pre className="diff-view">
      <code>
        {lines.map((line, idx) => (
          // eslint-disable-next-line react/no-array-index-key -- diff lines are static text re-rendered on full text swap; index is stable per fetch
          <span key={idx} className={`diff-line diff-line-${classify(line)}`}>
            {line}
            {"\n"}
          </span>
        ))}
      </code>
    </pre>
  );
}
