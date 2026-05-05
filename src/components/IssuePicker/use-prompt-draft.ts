import { useEffect, useMemo, useState } from "preact/hooks";
import { commands } from "../../lib/bindings";
import { DEFAULT_SPAWN_PROMPT, renderPrompt } from "../../lib/spawn-prompt";
import type { Issue } from "./types";

export interface PromptDraft {
  /// Resolved prompt for the highlighted issue: per-issue override (if any)
  /// → saved template → built-in default, with placeholders interpolated.
  resolvedPrompt: string;
  /// `true` when the highlighted issue has an override different from the
  /// saved template. Used by the spawn flow to pick `prompt_override`.
  isDirty: boolean;
  /// Override the prompt for one specific issue only. Empty/whitespace
  /// drops the override (revert to saved template).
  setOverride: (issueNumber: number, value: string) => void;
  /// Drop the override for `issueNumber`.
  reset: (issueNumber: number) => void;
  /// The raw template currently in effect for `issueNumber` (override or
  /// saved template), without placeholder interpolation. Used to seed the
  /// edit textarea.
  templateFor: (issueNumber: number) => string;
  /// Returns the override (rendered with placeholders filled) for this
  /// issue, or `null` if there is none. Used by the spawn flow to decide
  /// whether to send `prompt_override` to the backend.
  getOverrideFor: (issue: Issue) => string | null;
}

/// Loads the saved spawn prompt template once on mount, then tracks
/// per-issue overrides in a Map so navigating between issues preserves
/// each one's draft.
export function usePromptDraft(highlighted: Issue | null): PromptDraft {
  const [savedTemplate, setSavedTemplate] = useState<string | null>(null);
  const [overrides, setOverrides] = useState<Map<number, string>>(() => new Map());

  useEffect(() => {
    void (async () => {
      const result = await commands.getConfig();
      if (result.status === "ok") {
        setSavedTemplate(result.data.spawnPromptTemplate ?? null);
      }
    })();
  }, []);

  const baseTemplate = savedTemplate ?? DEFAULT_SPAWN_PROMPT;

  const templateFor = (issueNumber: number) => overrides.get(issueNumber) ?? baseTemplate;

  const resolvedPrompt = useMemo(() => {
    if (!highlighted) return "";
    const template = overrides.get(highlighted.number) ?? baseTemplate;
    return renderPrompt(template, highlighted.number, highlighted.title);
  }, [highlighted, baseTemplate, overrides]);

  const isDirty = !!highlighted && overrides.has(highlighted.number);

  const setOverride = (issueNumber: number, value: string) => {
    setOverrides((prev) => {
      const next = new Map(prev);
      if (value.trim().length === 0 || value === baseTemplate) {
        next.delete(issueNumber);
      } else {
        next.set(issueNumber, value);
      }
      return next;
    });
  };

  const reset = (issueNumber: number) => {
    setOverrides((prev) => {
      if (!prev.has(issueNumber)) return prev;
      const next = new Map(prev);
      next.delete(issueNumber);
      return next;
    });
  };

  const getOverrideFor = (issue: Issue): string | null => {
    const override = overrides.get(issue.number);
    if (override === undefined) return null;
    return renderPrompt(override, issue.number, issue.title);
  };

  return { resolvedPrompt, isDirty, setOverride, reset, templateFor, getOverrideFor };
}
