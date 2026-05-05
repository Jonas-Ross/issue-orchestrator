import { commands } from "./bindings";
import { DEFAULT_PTY_COLS, DEFAULT_PTY_ROWS } from "./constants";

/// Spawn a debug bash session in the worktree root. Errors are logged
/// rather than thrown — every callsite is fire-and-forget UI (sidebar
/// button, empty-state action, ⌘⇧B keymap) and there's no useful
/// recovery beyond the dev-tools log.
export async function spawnBash() {
  const result = await commands.ptySpawn(DEFAULT_PTY_COLS, DEFAULT_PTY_ROWS);
  if (result.status === "error") {
    console.error("ptySpawn failed:", result.error);
  }
}
