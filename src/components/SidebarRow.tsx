import { open } from "@tauri-apps/plugin-shell";
import { useState, useEffect } from "preact/hooks";
import { commands } from "../lib/bindings";
import type { SessionSummary } from "../lib/bindings";
import { copyToClipboard } from "../lib/clipboard";
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

function issueIdFromUrl(url: string | null): string | null {
  if (!url) return null;
  // GitHub: /issues/123 ; Jira: /browse/PROJ-123 ; Linear: /issue/ENG-456
  const m = url.match(/\/(?:issues|browse|issue)\/([A-Za-z0-9_-]+)(?:[/?#].*)?$/);
  return m ? m[1] : null;
}

export function SidebarRow({ session, collapsed }: Props) {
  const isActive = activeId.value === session.id;
  const isNeeds = session.status === "needs_input";
  const isShell = !session.issueUrl && !session.branch;
  const issueNum = issueIdFromUrl(session.issueUrl);
  const branchOrPath = session.branch ?? session.worktreePath ?? "—";

  const [replyValue, setReplyValue] = useState("");

  useEffect(() => {
    if (!isNeeds) setReplyValue("");
  }, [isNeeds]);

  const onSelect = () => {
    activeId.value = session.id;
  };

  const onReplyKeyDown = (e: KeyboardEvent) => {
    e.stopPropagation();
    if (e.key === "Enter") {
      if (e.isComposing) return;
      const text = replyValue;
      setReplyValue("");
      void commands.ptyWrite(session.id, text + "\n");
    } else if (e.key === "Escape") {
      setReplyValue("");
      (e.target as HTMLInputElement).blur();
    }
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
        action: () => void copyToClipboard(path),
      });
    }
    if (session.branch) {
      const branch = session.branch;
      items.push({
        label: "Copy branch name",
        action: () => void copyToClipboard(branch),
      });
    }
    if (session.issueUrl) {
      const url = session.issueUrl;
      items.push({
        label: "Copy issue link",
        action: () => void copyToClipboard(url),
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
        class={"sb-row sb-row-collapsed" + (isActive ? " active" : "") + (isNeeds ? " needs" : "")}
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
        <span class="sb-row-title" title={session.title}>
          {session.title}
        </span>
        {session.issueUrl && (
          <a
            class="sb-row-extlink"
            href={session.issueUrl}
            title="Open issue"
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

      {isNeeds && (
        <input
          class="sb-row-reply"
          type="text"
          value={replyValue}
          placeholder="Reply…"
          onInput={(e) => setReplyValue((e.target as HTMLInputElement).value)}
          onKeyDown={onReplyKeyDown}
          onClick={(e) => e.stopPropagation()}
        />
      )}
    </div>
  );
}
