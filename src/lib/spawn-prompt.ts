/// Built-in fallback used when neither a per-session override nor the
/// user's saved `spawnPromptTemplate` is set. Mirrored from
/// `DEFAULT_SPAWN_PROMPT` in `src-tauri/src/spawn.rs` — keep in sync.
export const DEFAULT_SPAWN_PROMPT =
  "Use the issue-team skill to implement issue #{issue_id} ({issue_title}).";

/// Mirrors Rust `render_prompt`. `{issue_number}` is accepted as a
/// back-compat alias for `{issue_id}` so templates saved before the
/// multi-provider switch keep working. Templates without placeholders
/// pass through unchanged.
export function renderPrompt(template: string, issueId: string, issueTitle: string): string {
  return template
    .split("{issue_id}")
    .join(issueId)
    .split("{issue_number}")
    .join(issueId)
    .split("{issue_title}")
    .join(issueTitle);
}
