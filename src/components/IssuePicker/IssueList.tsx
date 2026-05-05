import type { Ref } from "preact";
import { IssueRow } from "./IssueRow";
import type { Decision, Issue, IssueBody } from "./types";

interface Props {
  listRef: Ref<HTMLUListElement>;
  issues: Issue[];
  highlightedIndex: number;
  expanded: number | null;
  recommendation: Decision | null;
  spawning: number | null;
  bodies: Map<number, IssueBody>;
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
          key={issue.number}
          issue={issue}
          index={idx}
          isHighlighted={idx === highlightedIndex}
          isExpanded={expanded === issue.number}
          isAiPick={recommendation?.number === issue.number}
          recommendation={recommendation}
          spawning={spawning}
          body={bodies.get(issue.number)}
          onSpawn={onSpawn}
          onToggleExpand={onToggleExpand}
          onHighlight={onHighlight}
        />
      ))}
    </ul>
  );
}
