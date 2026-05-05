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
  setOverride: (issueId: string, value: string) => void;
  /// Drop the override for `issueId`.
  reset: (issueId: string) => void;
  /// The raw template currently in effect for `issueId` (override or
  /// saved template), without placeholder interpolation. Used to seed the
  /// edit textarea.
  templateFor: (issueId: string) => string;
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
  const [overrides, setOverrides] = useState<Map<string, string>>(() => new Map());

  useEffect(() => {
    void (async () => {
      const result = await commands.getConfig();
      if (result.status === "ok") {
        setSavedTemplate(result.data.spawnPromptTemplate ?? null);
      }
    })();
  }, []);

  const baseTemplate = savedTemplate ?? DEFAULT_SPAWN_PROMPT;

  const templateFor = (issueId: string) => overrides.get(issueId) ?? baseTemplate;

  const resolvedPrompt = useMemo(() => {
    if (!highlighted) return "";
    const template = overrides.get(highlighted.id) ?? baseTemplate;
    return renderPrompt(template, highlighted.id, highlighted.title);
  }, [highlighted, baseTemplate, overrides]);

  const isDirty = !!highlighted && overrides.has(highlighted.id);

  const setOverride = (issueId: string, value: string) => {
    setOverrides((prev) => {
      const next = new Map(prev);
      if (value.trim().length === 0 || value === baseTemplate) {
        next.delete(issueId);
      } else {
        next.set(issueId, value);
      }
      return next;
    });
  };

  const reset = (issueId: string) => {
    setOverrides((prev) => {
      if (!prev.has(issueId)) return prev;
      const next = new Map(prev);
      next.delete(issueId);
      return next;
    });
  };

  const getOverrideFor = (issue: Issue): string | null => {
    const override = overrides.get(issue.id);
    if (override === undefined) return null;
    return renderPrompt(override, issue.id, issue.title);
  };

  return { resolvedPrompt, isDirty, setOverride, reset, templateFor, getOverrideFor };
}
