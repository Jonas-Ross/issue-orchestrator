import { sessions, activeId } from "../state/sessions";
import { RightPane } from "./RightPane";
import { EmptyState } from "./EmptyState";

export function TerminalArea() {
  const list = sessions.value;
  const active = activeId.value;
  return (
    <div className="terminal-area">
      {list.length === 0 && <EmptyState />}
      {list.map((s) => (
        <RightPane
          key={s.id}
          sessionId={s.id}
          active={s.id === active}
          worktreePath={s.worktreePath ?? null}
        />
      ))}
    </div>
  );
}
