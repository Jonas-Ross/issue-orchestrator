import { useEffect, useState } from "preact/hooks";
import { commands } from "../../lib/bindings";
import type { IssueState } from "./types";

export function useIssuesList(selectedRepo: string | null) {
  const [issues, setIssues] = useState<IssueState>({ tag: "idle" });

  useEffect(() => {
    if (!selectedRepo) return;
    let cancelled = false;
    setIssues({ tag: "loading" });
    void (async () => {
      const result = await commands.listIssues(selectedRepo);
      if (cancelled) return;
      setIssues(
        result.status === "error"
          ? { tag: "error", message: result.error }
          : { tag: "ok", issues: result.data },
      );
    })();
    return () => {
      cancelled = true;
    };
  }, [selectedRepo]);

  return [issues, setIssues] as const;
}
