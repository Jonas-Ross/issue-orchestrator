import type { Ref } from "preact";
import { IssueRow } from "./IssueRow";
import type { Decision, Issue, IssueBody } from "./types";

interface Props {
  listRef: Ref<HTMLUListElement>;
  issues: Issue[];
  highlightedIndex: number;
  expanded: string | null;
  recommendation: Decision | null;
  spawning: string | null;
  bodies: Map<string, IssueBody>;
  onSpawn: (issue: Issue) => void;
  onToggleExpand: (issue: Issue) => void;
  onHighlight: (index: number) => void;
}

export function IssueList({
  listRef,
  issues,
  highlightedIndex,
  expanded,
  recommendation,
  spawning,
  bodies,
  onSpawn,
  onToggleExpand,
  onHighlight,
}: Props) {
  return (
    <ul class="issue-list" ref={listRef}>
      {issues.length === 0 && (
        <li class="hint" style={{ padding: "12px 16px", borderBottom: "none" }}>
          No matching issues.
        </li>
      )}
      {issues.map((issue, idx) => (
        <IssueRow
          key={issue.id}
          issue={issue}
          index={idx}
          isHighlighted={idx === highlightedIndex}
          isExpanded={expanded === issue.id}
          isAiPick={recommendation?.id === issue.id}
          recommendation={recommendation}
          spawning={spawning}
          body={bodies.get(issue.id)}
          onSpawn={onSpawn}
          onToggleExpand={onToggleExpand}
          onHighlight={onHighlight}
        />
      ))}
    </ul>
  );
}
