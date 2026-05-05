import { useEffect, useState } from "preact/hooks";
import type { Issue } from "./types";

export function useHighlightedIssue(
  filteredIssues: Issue[],
  listRef: { current: HTMLUListElement | null },
) {
  const [highlightedIndex, setHighlightedIndex] = useState(0);

  // Clamp highlight when the filtered set shrinks (search typing). Reads
  // the index via the functional setter so it doesn't need to be a
  // dependency — otherwise this effect would re-fire on every arrow key.
  useEffect(() => {
    setHighlightedIndex((i) => {
      if (filteredIssues.length === 0) return 0;
      if (i >= filteredIssues.length) return filteredIssues.length - 1;
      return i;
    });
  }, [filteredIssues]);

  // Auto-scroll the highlighted issue into view as the user arrows.
  useEffect(() => {
    if (filteredIssues.length === 0 || !listRef.current) return;
    const target = filteredIssues[highlightedIndex];
    if (!target) return;
    const el = listRef.current.querySelector(`[data-issue-id="${CSS.escape(target.id)}"]`);
    if (el && "scrollIntoView" in el) {
      (el as HTMLElement).scrollIntoView({ block: "nearest" });
    }
  }, [highlightedIndex, filteredIssues, listRef]);

  return [highlightedIndex, setHighlightedIndex] as const;
}
