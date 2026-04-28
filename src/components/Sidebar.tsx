import { commands } from "../lib/bindings";
import { sessions, sessionsByRepo, SHELL_BUCKET } from "../state/sessions";
import { repos } from "../state/repos";
import { sidebarCollapsed, toggleSidebar } from "../state/sidebar";
import { openSettings } from "../state/settings";
import { SidebarRow } from "./SidebarRow";
import { RepoDrawer } from "./RepoDrawer";
import { AddRepoButton } from "./AddRepoButton";
import { StatusDot } from "./StatusDot";

async function spawnBash() {
  const result = await commands.ptySpawn(80, 24);
  if (result.status === "error") {
    console.error("ptySpawn failed:", result.error);
  }
}

export function Sidebar() {
  const collapsed = sidebarCollapsed.value;
  const list = sessions.value;
  const repoList = repos.value;
  const grouped = sessionsByRepo.value;
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

  const shellSessions = grouped.get(SHELL_BUCKET) ?? [];

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

      <div class="sb-rows">
        {repoList.length === 0 ? (
          <div class="sb-empty">
            <p class="sb-empty-title">No repos yet</p>
            <AddRepoButton variant="primary" />
          </div>
        ) : (
          <>
            {repoList.map((r) => (
              <RepoDrawer
                key={r.name}
                repo={r}
                sessions={grouped.get(r.name) ?? []}
              />
            ))}

            {shellSessions.length > 0 && (
              <div class="repo-drawer expanded shell-drawer">
                <div class="repo-drawer-header">
                  <span class="repo-drawer-caret">▾</span>
                  <span class="repo-drawer-name">Debug shells</span>
                  <span class="repo-drawer-count">{shellSessions.length}</span>
                </div>
                <div class="repo-drawer-body">
                  {shellSessions.map((s) => (
                    <SidebarRow key={s.id} session={s} collapsed={false} />
                  ))}
                </div>
              </div>
            )}

            <AddRepoButton variant="tile" />
          </>
        )}
      </div>

      <footer class="sb-footer">
        <span class="sb-footer-dot" />
        <span>hooks · up</span>
        <span class="sb-spacer" />
        <button
          type="button"
          class="sb-footer-btn"
          title="Settings (⌘,)"
          onClick={() => openSettings()}
        >
          ⚙
        </button>
        <button
          type="button"
          class="sb-footer-btn sb-action-shell"
          title="Debug bash (⌘⇧B)"
          onClick={() => void spawnBash()}
        >
          ⌘
        </button>
      </footer>
    </aside>
  );
}
