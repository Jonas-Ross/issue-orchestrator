import { spawnBash } from "../lib/spawn-bash";
import { openPicker } from "../state/picker";
import { Kbd } from "./Kbd";

export function EmptyState() {
  return (
    <div class="empty-state">
      <div class="empty-card">
        <div class="empty-kicker">0 sessions · ready</div>
        <div class="empty-title">Pick an issue. Spawn a session.</div>
        <div class="empty-sub">
          Each one runs Claude in its own git worktree. Status flips back here when something needs
          you.
        </div>
        <div class="empty-actions">
          <button type="button" class="empty-action primary" onClick={() => openPicker()}>
            <span class="empty-glyph empty-glyph-accent">＋</span>
            <span class="empty-action-text">
              <div class="empty-action-title">Pick an issue</div>
              <div class="empty-action-sub">Browse open issues from a configured repo</div>
            </span>
            <Kbd>⌘ N</Kbd>
          </button>
          <button type="button" class="empty-action" onClick={() => void spawnBash()}>
            <span class="empty-glyph">⌘</span>
            <span class="empty-action-text">
              <div class="empty-action-title">Open a debug bash tab</div>
              <div class="empty-action-sub">Plain shell session in the worktree root</div>
            </span>
            <Kbd>⌘ ⇧ B</Kbd>
          </button>
        </div>
      </div>
    </div>
  );
}
