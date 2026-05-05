import { useState } from "preact/hooks";
import { commands } from "../../lib/bindings";
import type { Issue, IssueBody } from "./types";

export function useIssueBodies(selectedRepo: string | null) {
  const [expanded, setExpanded] = useState<string | null>(null);
  const [bodies, setBodies] = useState<Map<string, IssueBody>>(new Map());

  const toggleExpand = async (issue: Issue) => {
    if (expanded === issue.id) {
      setExpanded(null);
      return;
    }
    setExpanded(issue.id);
    if (!selectedRepo || bodies.has(issue.id)) return;
    setBodies((prev) => new Map(prev).set(issue.id, "loading"));
    const result = await commands.getIssueBody(selectedRepo, issue.id);
    setBodies((prev) =>
      new Map(prev).set(
        issue.id,
        result.status === "error" ? { error: result.error } : result.data,
      ),
    );
  };

  return { expanded, bodies, toggleExpand };
}
