import { useEffect, useState } from "preact/hooks";
import { commands } from "../../lib/bindings";
import type { Decision, Issue } from "./types";

export function useIssueRecommendation(
  selectedRepo: string | null,
  filteredIssues: Issue[],
  listRef: { current: HTMLUListElement | null },
) {
  const [recommendation, setRecommendation] = useState<Decision | null>(null);
  const [recommending, setRecommending] = useState(false);
  const [recoError, setRecoError] = useState<string | null>(null);

  // Reset when the user switches repos.
  useEffect(() => {
    setRecommendation(null);
    setRecoError(null);
  }, [selectedRepo]);

  // Scroll the AI-picked issue into view once it's resolved.
  useEffect(() => {
    if (!recommendation || !listRef.current) return;
    const el = listRef.current.querySelector(`[data-issue-id="${CSS.escape(recommendation.id)}"]`);
    if (el && "scrollIntoView" in el) {
      (el as HTMLElement).scrollIntoView({ block: "nearest", behavior: "smooth" });
    }
  }, [recommendation, filteredIssues, listRef]);

  const onDecide = async () => {
    if (!selectedRepo || recommending) return;
    setRecommending(true);
    setRecoError(null);
    const result = await commands.decideNextIssue(selectedRepo);
    setRecommending(false);
    if (result.status === "error") {
      setRecoError(result.error);
      return;
    }
    setRecommendation(result.data);
  };

  return { recommendation, recommending, recoError, onDecide };
}
