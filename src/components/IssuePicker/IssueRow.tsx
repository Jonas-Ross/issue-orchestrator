import { open } from "@tauri-apps/plugin-shell";
import { copyToClipboard } from "../../lib/clipboard";
import { openContextMenu } from "../../state/context-menu";
import type { Decision, Issue, IssueBody } from "./types";

interface Props {
  issue: Issue;
  index: number;
  isHighlighted: boolean;
  isExpanded: boolean;
  isAiPick: boolean;
  recommendation: Decision | null;
  spawning: string | null;
  body: IssueBody | undefined;
  onSpawn: (issue: Issue) => void;
  onToggleExpand: (issue: Issue) => void;
  onHighlight: (index: number) => void;
}

function renderBody(body: IssueBody | undefined): string {
  if (body === undefined || body === "loading") return "Loading…";
  if (typeof body === "string") return body || "(no body)";
  return `Failed to load body: ${body.error}`;
}

function showContextMenu(e: MouseEvent, issue: Issue) {
  e.preventDefault();
  e.stopPropagation();
  openContextMenu({
    x: e.clientX,
    y: e.clientY,
    items: [
      { label: "Open issue ↗", action: () => void open(issue.url) },
      { label: "Copy issue link", action: () => void copyToClipboard(issue.url) },
      {
        label: "Copy branch name",
        action: () => void copyToClipboard(`issue-${branchSafe(issue.id)}`),
      },
    ],
  });
}

/// Mirrors the Rust `sanitize_branch` helper so the "Copy branch name"
/// context-menu action lines up with what the spawn flow actually
/// computes. Lowercase, keep `[a-z0-9._-]`, collapse `-` runs, trim.
function branchSafe(id: string): string {
  let out = "";
  let lastDash = false;
  for (const ch of id.toLowerCase()) {
    const isDash = ch === "-";
    const keep = /[a-z0-9._-]/.test(ch) && !isDash;
    if (keep) {
      out += ch;
      lastDash = false;
    } else if (!lastDash) {
      out += "-";
      lastDash = true;
    }
  }
  const trimmed = out.replace(/^-+|-+$/g, "");
  return trimmed.length === 0 ? "issue" : trimmed;
}

export function IssueRow({
  issue,
  index,
  isHighlighted,
  isExpanded,
  isAiPick,
  recommendation,
  spawning,
  body,
  onSpawn,
  onToggleExpand,
  onHighlight,
}: Props) {
  const isSpawning = spawning === issue.id;
  return (
    <li
      class={
        "issue" +
        (isSpawning ? " spawning" : "") +
        (isAiPick ? " ai-pick" : "") +
        (isHighlighted ? " highlighted" : "")
      }
      data-issue-id={issue.id}
      style={{ gridTemplateColumns: "auto auto 1fr auto auto" }}
      onClick={() => onSpawn(issue)}
      onContextMenu={(e) => showContextMenu(e, issue)}
      onMouseEnter={() => onHighlight(index)}
    >
      <button
        type="button"
        class="issue-caret"
        title={isExpanded ? "Collapse" : "Show body"}
        onClick={(e) => {
          e.stopPropagation();
          onToggleExpand(issue);
        }}
      >
        {isExpanded ? "▾" : "▸"}
      </button>
      <a
        class="issue-number"
        href={issue.url}
        title="Open issue"
        onClick={(e) => {
          e.preventDefault();
          e.stopPropagation();
          void open(issue.url);
        }}
      >
        #{issue.id}
      </a>
      <span class="issue-title">{issue.title}</span>
      <span class="issue-labels">
        {isAiPick && <span class="label ai-pick">AI pick</span>}
        {issue.labels.map((l) => (
          <span key={l} class="label">
            {l}
          </span>
        ))}
      </span>
      {isSpawning && <span class="spinner">Spawning…</span>}
      {isAiPick && recommendation && (
        <span class="issue-reasoning">{recommendation.reasoning}</span>
      )}
      {isExpanded && <pre class="issue-body">{renderBody(body)}</pre>}
    </li>
  );
}
