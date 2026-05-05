import { useState } from "preact/hooks";
import { commands } from "../../lib/bindings";
import type { Issue, IssueBody } from "./types";

export function useIssueBodies(selectedRepo: string | null) {
  const [expanded, setExpanded] = useState<number | null>(null);
  const [bodies, setBodies] = useState<Map<number, IssueBody>>(new Map());

  const toggleExpand = async (issue: Issue) => {
    if (expanded === issue.number) {
      setExpanded(null);
      return;
    }
    setExpanded(issue.number);
    if (!selectedRepo || bodies.has(issue.number)) return;
    setBodies((prev) => new Map(prev).set(issue.number, "loading"));
    const result = await commands.getIssueBody(selectedRepo, issue.number);
    setBodies((prev) =>
      new Map(prev).set(
        issue.number,
        result.status === "error" ? { error: result.error } : result.data,
      ),
    );
  };

  return { expanded, bodies, toggleExpand };
}
