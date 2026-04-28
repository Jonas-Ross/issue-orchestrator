import { open } from "@tauri-apps/plugin-shell";
import { commands } from "../lib/bindings";
import type { SessionSummary } from "../lib/bindings";
import { activeId } from "../state/sessions";
import { openContextMenu } from "../state/context-menu";
import type { ContextMenuItem } from "../state/context-menu";
import { StatusDot } from "./StatusDot";
import { ActivityIcon } from "./ActivityIcon";

interface Props {
  session: SessionSummary;
  collapsed: boolean;
}

const ACTIVITY_LABEL: Record<SessionSummary["status"], string> = {
  spawning: "Spawning…",
  running: "Working",
  needs_input: "Awaiting input",
  idle: "Idle",
  exited: "Exited",
};

function issueNumberFromUrl(url: string | null): string | null {
  if (!url) return null;
  const m = url.match(/\/issues\/(\d+)(?:[/?#].*)?$/);
  return m ? m[1] : null;
}

export function SidebarRow({ session, collapsed }: Props) {
  const isActive = activeId.value === session.id;
  const isNeeds = session.status === "needs_input";
  const isShell = !session.issueUrl && !session.branch;
  const issueNum = issueNumberFromUrl(session.issueUrl);
  const branchOrPath = session.branch ?? session.worktreePath ?? "—";

  const onSelect = () => {
    activeId.value = session.id;
  };

  const onContextMenu = (e: MouseEvent) => {
    e.preventDefault();
    const items: ContextMenuItem[] = [];
    items.push({ label: "Switch to", action: onSelect });
    if (session.worktreePath) {
      const path = session.worktreePath;
      items.push({
        label: "Reveal worktree in Finder",
        action: () => void open(path).catch(() => {}),
      });
      items.push({
        label: "Copy worktree path",
        action: () => void navigator.clipboard.writeText(path),
      });
    }
    if (session.branch) {
      const branch = session.branch;
      items.push({
        label: "Copy branch name",
        action: () => void navigator.clipboard.writeText(branch),
      });
    }
    if (session.issueUrl) {
      const url = session.issueUrl;
      items.push({
        label: "Copy issue link",
        action: () => void navigator.clipboard.writeText(url),
      });
      items.push({ label: "Open issue in browser", action: () => void open(url) });
    }
    items.push({ separator: true });
    items.push({
      label: "Close session",
      action: () => void commands.ptyKill(session.id),
    });
    openContextMenu({ x: e.clientX, y: e.clientY, items });
  };

  if (collapsed) {
    return (
      <div
        class={
          "sb-row sb-row-collapsed" +
          (isActive ? " active" : "") +
          (isNeeds ? " needs" : "")
        }
        onClick={onSelect}
        onContextMenu={onContextMenu}
        title={`#${issueNum ?? "—"} ${session.title}`}
      >
        <StatusDot status={session.status} size={9} pulse={isNeeds} />
      </div>
    );
  }

  return (
    <div
      class={
        "sb-row" +
        (isActive ? " active" : "") +
        (isNeeds ? " needs" : "") +
        (session.status === "exited" ? " exited" : "")
      }
      onClick={onSelect}
      onContextMenu={onContextMenu}
    >
      <div class="sb-row-line1">
        <StatusDot status={session.status} size={7} pulse={isNeeds} />
        {issueNum && <span class="sb-row-num">#{issueNum}</span>}
        <span class="sb-row-title" title={session.title}>{session.title}</span>
        {session.issueUrl && (
          <a
            class="sb-row-extlink"
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
          class="sb-row-close"
          title="Close session"
          onClick={(e) => {
            e.stopPropagation();
            void commands.ptyKill(session.id);
          }}
        >
          ×
        </button>
      </div>

      <div class="sb-row-line2">
        <ActivityIcon status={session.status} isShell={isShell} />
        <span class="sb-row-activity">{ACTIVITY_LABEL[session.status]}</span>
      </div>

      {!isShell && (
        <div class="sb-row-line3">
          <span class="sb-row-branch" title={branchOrPath}>
            {branchOrPath}
          </span>
        </div>
      )}
    </div>
  );
}
