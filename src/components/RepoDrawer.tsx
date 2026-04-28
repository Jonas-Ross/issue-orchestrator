import { isDrawerExpanded, removeRepoByName, toggleDrawer } from "../state/repos";
import { openPicker } from "../state/picker";
import { openContextMenu } from "../state/context-menu";
import type { ContextMenuItem } from "../state/context-menu";
import type { RepoEntry, SessionSummary } from "../lib/bindings";
import { SidebarRow } from "./SidebarRow";

interface Props {
  repo: RepoEntry;
  sessions: SessionSummary[];
}

export function RepoDrawer({ repo, sessions }: Props) {
  const expanded = isDrawerExpanded(repo.name);
  const needsCount = sessions.filter((s) => s.status === "needs_input").length;

  const onRemove = async () => {
    if (sessions.length > 0) {
      // Backend refuses while sessions live. Surface it as a confirm so
      // users know what to do next.
      alert(
        `Close the ${sessions.length} session${sessions.length === 1 ? "" : "s"} in "${repo.name}" before removing the repo.`,
      );
      return;
    }
    try {
      await removeRepoByName(repo.name);
    } catch (e) {
      alert(`Could not remove repo: ${e instanceof Error ? e.message : e}`);
    }
  };

  const onContextMenu = (e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const items: ContextMenuItem[] = [
      {
        label: "Copy repo path",
        action: () => void navigator.clipboard.writeText(repo.path),
      },
      { separator: true },
      { label: "Remove repo…", action: () => void onRemove() },
    ];
    openContextMenu({ x: e.clientX, y: e.clientY, items });
  };

  return (
    <div class={`repo-drawer${expanded ? " expanded" : " collapsed"}`}>
      <div
        class="repo-drawer-header"
        onClick={() => toggleDrawer(repo.name)}
        onContextMenu={onContextMenu}
        title={repo.path}
      >
        <span class="repo-drawer-caret">{expanded ? "▾" : "▸"}</span>
        <span class="repo-drawer-name">{repo.name}</span>
        <span class="repo-drawer-count">
          {needsCount > 0 ? (
            <span class="repo-drawer-needs">{needsCount}</span>
          ) : (
            sessions.length
          )}
        </span>
        <button
          type="button"
          class="repo-drawer-new"
          title={`New session in ${repo.name}`}
          onClick={(e) => {
            e.stopPropagation();
            openPicker(repo.name);
          }}
        >
          ＋
        </button>
      </div>

      {expanded && (
        <div class="repo-drawer-body">
          {sessions.length === 0 ? (
            <div class="repo-drawer-empty">No sessions yet</div>
          ) : (
            sessions.map((s) => (
              <SidebarRow key={s.id} session={s} collapsed={false} />
            ))
          )}
        </div>
      )}
    </div>
  );
}
