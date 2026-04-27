import { sessions, activeId } from "../state/sessions";
import { TerminalView } from "./TerminalView";

export function TerminalArea() {
  const list = sessions.value;
  const active = activeId.value;
  return (
    <div className="terminal-area">
      {list.length === 0 && (
        <div className="empty">No sessions. Click + to spawn one.</div>
      )}
      {list.map((s) => (
        <TerminalView key={s.id} sessionId={s.id} active={s.id === active} />
      ))}
    </div>
  );
}
