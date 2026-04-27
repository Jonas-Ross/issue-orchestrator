import { commands } from "../lib/bindings";
import { sessions } from "../state/sessions";
import { openPicker } from "../state/picker";
import { Tab } from "./Tab";

export function TabStrip() {
  return (
    <div class="tab-strip">
      {sessions.value.map((s) => (
        <Tab key={s.id} session={s} />
      ))}
      <button
        type="button"
        class="add-tab"
        title="Spawn issue session"
        onClick={() => openPicker()}
      >
        +
      </button>
      <button
        type="button"
        class="add-tab secondary"
        title="Spawn a plain bash tab (debug)"
        onClick={() => void spawnBash()}
      >
        ⌘
      </button>
    </div>
  );
}

async function spawnBash() {
  const result = await commands.ptySpawn(80, 24);
  if (result.status === "error") {
    console.error("ptySpawn failed:", result.error);
  }
}
