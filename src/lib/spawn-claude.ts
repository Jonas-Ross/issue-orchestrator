import { commands } from "./bindings";
import { DEFAULT_PTY_COLS, DEFAULT_PTY_ROWS } from "./constants";

/// Spawn a scratch `claude` session — repo-scoped when `repoName` is
/// passed (sidebar drawer "+" button), unbucketed otherwise (footer
/// button). Errors are logged rather than thrown to mirror
/// `spawnBash`: callsites are fire-and-forget UI and there's no
/// useful recovery beyond the dev-tools log.
export async function spawnClaude(repoName?: string) {
  const result = await commands.claudeSpawn(repoName ?? null, DEFAULT_PTY_COLS, DEFAULT_PTY_ROWS);
  if (result.status === "error") {
    console.error("claudeSpawn failed:", result.error);
  }
}
