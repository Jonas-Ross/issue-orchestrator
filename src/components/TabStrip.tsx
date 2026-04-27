import { commands } from "../lib/bindings";
import { sessions } from "../state/sessions";
import { Tab } from "./Tab";

export function TabStrip() {
  return (
    <div className="tab-strip">
      {sessions.value.map((s) => (
        <Tab key={s.id} session={s} />
      ))}
      <button
        type="button"
        className="add-tab"
        title="New session"
        onClick={() => void spawnBash()}
      >
        +
      </button>
    </div>
  );
}

async function spawnBash() {
  // M2: '+' spawns a bare bash. M4 will swap this for the issue picker
  // modal so the same button drives the orchestrated `claude` flow.
  const result = await commands.ptySpawn(80, 24);
  if (result.status === "error") {
    console.error("ptySpawn failed:", result.error);
  }
  // The actor's SessionAdded event handles inserting into the sessions
  // signal, so nothing else to do here.
}
