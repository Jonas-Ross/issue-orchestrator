import { useState } from "preact/hooks";
import type { Issue } from "./types";
import type { PromptDraft } from "./use-prompt-draft";

/// Collapsed: a one-line preview of the resolved prompt for the
/// highlighted issue. Expanded: a textarea so the user can override the
/// template for *just* this issue without touching the saved one.
export function IssuePromptPreview({ issue, draft }: { issue: Issue | null; draft: PromptDraft }) {
  const [expanded, setExpanded] = useState(false);
  const [editing, setEditing] = useState(false);
  if (!issue) return null;

  return (
    <div class="prompt-preview">
      <button
        type="button"
        class="prompt-preview-summary"
        onClick={() => setExpanded((v) => !v)}
        title="Toggle prompt preview"
      >
        <span class="prompt-preview-caret">{expanded ? "▾" : "▸"}</span>
        <span class="prompt-preview-label">Prompt:</span>
        <span class="prompt-preview-text">{draft.resolvedPrompt}</span>
        {draft.isDirty && <span class="prompt-preview-badge">edited</span>}
      </button>
      {expanded && (
        <div class="prompt-preview-body">
          {editing ? (
            <textarea
              class="prompt-textarea"
              rows={4}
              spellcheck={false}
              autoFocus
              value={draft.templateFor(issue.id)}
              onInput={(e) => draft.setOverride(issue.id, (e.target as HTMLTextAreaElement).value)}
              onBlur={() => setEditing(false)}
            />
          ) : (
            <pre class="prompt-preview-resolved">{draft.resolvedPrompt}</pre>
          )}
          <div class="prompt-preview-toolbar">
            <button type="button" class="prompt-btn" onClick={() => setEditing((v) => !v)}>
              {editing ? "Done" : "Edit"}
            </button>
            <button
              type="button"
              class="prompt-btn"
              disabled={!draft.isDirty}
              onClick={() => {
                draft.reset(issue.id);
                setEditing(false);
              }}
            >
              Reset
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
