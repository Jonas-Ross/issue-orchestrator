import type { IssueState } from "./types";

interface Props {
  selectedRepo: string | null;
  issues: IssueState;
}

export function IssuesStatusBanner({ selectedRepo, issues }: Props) {
  if (!selectedRepo) return null;
  if (issues.tag === "loading") return <p class="hint">Loading issues…</p>;
  if (issues.tag === "error") {
    return <p class="error">Failed to load issues: {issues.message}</p>;
  }
  if (issues.tag === "ok" && issues.issues.length === 0) {
    return <p class="hint">No open issues.</p>;
  }
  return null;
}
