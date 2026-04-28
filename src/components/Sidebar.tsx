import { commands } from "../lib/bindings";
import { sessions } from "../state/sessions";
import { openPicker } from "../state/picker";
import { sidebarCollapsed, toggleSidebar } from "../state/sidebar";
import { SidebarRow } from "./SidebarRow";
import { StatusDot } from "./StatusDot";
import { Kbd } from "./Kbd";

async function spawnBash() {
  const result = await commands.ptySpawn(80, 24);
  if (result.status === "error") {
    console.error("ptySpawn failed:", result.error);
  }
}

export function Sidebar() {
  const collapsed = sidebarCollapsed.value;
  const list = sessions.value;
  const counts = {
    needs: list.filter((s) => s.status === "needs_input").length,
    running: list.filter((s) => s.status === "running").length,
    idle: list.filter((s) => s.status === "idle").length,
  };

  if (collapsed) {
    return (
      <aside class="sb sb-collapsed">
        <button
          type="button"
          class="sb-iconbtn"
          title="Expand sidebar (⌘B)"
          onClick={toggleSidebar}
        >
          ›
        </button>
        <button
          type="button"
          class="sb-iconbtn"
          title="New session (⌘N)"
          onClick={() => openPicker()}
        >
          ＋
        </button>
        {counts.needs > 0 && (
          <div class="sb-needs-pill" title={`${counts.needs} need you`}>
            {counts.needs}
          </div>
        )}
        <div class="sb-rows">
          {list.map((s) => (
            <SidebarRow key={s.id} session={s} collapsed />
          ))}
        </div>
        <button
          type="button"
          class="sb-iconbtn sb-shell"
          title="Debug bash (⌘⇧B)"
          onClick={() => void spawnBash()}
        >
          ⌘
        </button>
      </aside>
    );
  }

  return (
    <aside class="sb">
      <header class="sb-header">
        <div class="sb-header-row">
          <span class="sb-header-label">Sessions · {list.length}</span>
          <button
            type="button"
            class="sb-collapse-btn"
            title="Collapse sidebar (⌘B)"
            onClick={toggleSidebar}
          >
            ‹
          </button>
        </div>
        <div class="sb-aggregate">
          {counts.needs > 0 ? (
            <span class="sb-aggregate-needs">
              <StatusDot status="needs_input" size={6} pulse />
              {counts.needs} need you
            </span>
          ) : (
            <span class="sb-aggregate-quiet">all caught up</span>
          )}
          <span class="sb-spacer" />
          <span class="sb-aggregate-tally" title="Running">
            <StatusDot status="running" size={5} /> {counts.running}
          </span>
          <span class="sb-aggregate-tally" title="Idle">
            <StatusDot status="idle" size={5} /> {counts.idle}
          </span>
        </div>
      </header>

      <div class="sb-actions">
        <button
          type="button"
          class="sb-action-new"
          onClick={() => openPicker()}
          title="Spawn a session from a GitHub issue"
        >
          <span class="sb-action-glyph">＋</span>
          <span class="sb-action-label">New session</span>
          <Kbd>⌘ N</Kbd>
        </button>
        <button
          type="button"
          class="sb-action-shell"
          onClick={() => void spawnBash()}
          title="Spawn a plain bash tab (⌘⇧B)"
        >
          ⌘
        </button>
      </div>

      <div class="sb-rows">
        {list.map((s) => (
          <SidebarRow key={s.id} session={s} collapsed={false} />
        ))}
      </div>

      <footer class="sb-footer">
        <span class="sb-footer-dot" />
        <span>hooks · up</span>
      </footer>
    </aside>
  );
}
