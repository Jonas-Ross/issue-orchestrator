import { useEffect } from "preact/hooks";
import type { Issue } from "./types";

interface Args {
  filteredIssues: Issue[];
  highlightedIndex: number;
  setHighlightedIndex: (updater: (i: number) => number) => void;
  onSpawn: (issue: Issue) => void;
}

// Capture phase so we beat the global keymap. Modifier-key combos pass
// through so ⌘N / ⌘W / etc. still work over the picker.
export function usePickerKeyboard({
  filteredIssues,
  highlightedIndex,
  setHighlightedIndex,
  onSpawn,
}: Args) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      if (e.key === "ArrowDown") {
        if (filteredIssues.length === 0) return;
        e.preventDefault();
        e.stopPropagation();
        setHighlightedIndex((i) => (i + 1) % filteredIssues.length);
      } else if (e.key === "ArrowUp") {
        if (filteredIssues.length === 0) return;
        e.preventDefault();
        e.stopPropagation();
        setHighlightedIndex((i) => (i - 1 + filteredIssues.length) % filteredIssues.length);
      } else if (e.key === "Enter") {
        // Don't hijack Enter inside the search input — that would feel
        // broken if the user is mid-type. Trigger only when focus is
        // outside an input, OR explicitly on the issue search field.
        const tgt = e.target as HTMLElement | null;
        const inInput = tgt?.tagName === "INPUT" || tgt?.tagName === "TEXTAREA";
        if (inInput && tgt?.classList.contains("issue-search") === false) return;
        const target = filteredIssues[highlightedIndex];
        if (!target) return;
        e.preventDefault();
        e.stopPropagation();
        onSpawn(target);
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [filteredIssues, highlightedIndex, setHighlightedIndex, onSpawn]);
}
