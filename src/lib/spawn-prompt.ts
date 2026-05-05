/// Built-in fallback used when neither a per-session override nor the
/// user's saved `spawnPromptTemplate` is set. Mirrored from
/// `DEFAULT_SPAWN_PROMPT` in `src-tauri/src/spawn.rs` — keep in sync.
export const DEFAULT_SPAWN_PROMPT =
  "Use the issue-team skill to implement issue #{issue_number} ({issue_title}).";

/// Mirrors Rust `render_prompt`: literal `{issue_number}` / `{issue_title}`
/// substitution, no other interpolation. Templates without placeholders
/// pass through unchanged.
export function renderPrompt(template: string, issueNumber: number, issueTitle: string): string {
  return template
    .split("{issue_number}")
    .join(String(issueNumber))
    .split("{issue_title}")
    .join(issueTitle);
}
