import { open } from "@tauri-apps/plugin-shell";
import { commands } from "../lib/bindings";
import type { SessionSummary } from "../lib/bindings";
import { activeId } from "../state/sessions";
import { openContextMenu } from "../state/context-menu";
import type { ContextMenuItem } from "../state/context-menu";

interface Props {
  session: SessionSummary;
}

export function Tab({ session }: Props) {
  const isActive = activeId.value === session.id;
  const onContextMenu = (e: MouseEvent) => {
    e.preventDefault();
    const items: ContextMenuItem[] = [];
    if (session.issueUrl) {
      const url = session.issueUrl;
      items.push({ label: "Open issue ↗", action: () => void open(url) });
    }
    if (session.branch) {
      const branch = session.branch;
      items.push({
        label: "Copy branch name",
        action: () => void navigator.clipboard.writeText(branch),
      });
    }
    if (session.worktreePath) {
      const path = session.worktreePath;
      items.push({
        label: "Copy worktree path",
        action: () => void navigator.clipboard.writeText(path),
      });
    }
    if (items.length) items.push({ separator: true } as const);
    items.push({
      label: "Kill session",
      action: () => void commands.ptyKill(session.id),
    });
    openContextMenu({ x: e.clientX, y: e.clientY, items });
  };

  return (
    <div
      className={`tab${isActive ? " active" : ""}`}
      onClick={() => {
        activeId.value = session.id;
      }}
      onContextMenu={onContextMenu}
    >
      <span className={`status status-${session.status}`} />
      <span className="title">{session.title}</span>
      {session.issueUrl && (
        <a
          class="tab-link"
          href={session.issueUrl}
          title="Open issue on GitHub"
          onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
            void open(session.issueUrl!);
          }}
        >
          ↗
        </a>
      )}
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
