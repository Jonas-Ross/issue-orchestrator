import { commands } from "../lib/bindings";
import type { SessionSummary } from "../lib/bindings";
import { activeId } from "../state/sessions";

interface Props {
  session: SessionSummary;
}

export function Tab({ session }: Props) {
  const isActive = activeId.value === session.id;
  return (
    <div
      className={`tab${isActive ? " active" : ""}`}
      onClick={() => {
        activeId.value = session.id;
      }}
    >
      <span className={`status status-${session.status}`} />
      <span className="title">{session.title}</span>
      <button
        type="button"
        className="close"
        title="Kill session"
        onClick={(e) => {
          e.stopPropagation();
          void commands.ptyKill(session.id);
        }}
      >
        ×
      </button>
    </div>
  );
}
