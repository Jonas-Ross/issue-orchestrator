import type { Decision, Issue } from "../../lib/bindings";

export type IssueState =
  | { tag: "idle" }
  | { tag: "loading" }
  | { tag: "ok"; issues: Issue[] }
  | { tag: "error"; message: string };

export type IssueBody = string | "loading" | { error: string };

export type { Decision, Issue };
