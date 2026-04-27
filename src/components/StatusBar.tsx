import { activeSession } from "../state/sessions";
import { StatusDot } from "./StatusDot";

const ACTIVITY_LABEL = {
  spawning: "Spawning…",
  running: "Working",
  needs_input: "Awaiting input",
  idle: "Idle",
  exited: "Exited",
} as const;

export function StatusBar() {
  const s = activeSession.value;
  if (!s) return null;
  const isNeeds = s.status === "needs_input";
  return (
    <div class="status-bar">
      <StatusDot status={s.status} size={6} />
      <span class="status-bar-title">{s.title}</span>
      {s.branch && (
        <>
          <span class="status-bar-sep">›</span>
          <span class="status-bar-branch">{s.branch}</span>
        </>
      )}
      <span class="status-bar-spacer" />
      <span
        class={"status-bar-activity" + (isNeeds ? " needs" : "")}
      >
        {ACTIVITY_LABEL[s.status]}
      </span>
    </div>
  );
}
