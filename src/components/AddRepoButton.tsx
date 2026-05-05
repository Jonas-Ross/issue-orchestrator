import { useState } from "preact/hooks";
import { open } from "@tauri-apps/plugin-dialog";
import { addRepoByPath } from "../state/repos";
import { settings } from "../state/settings";
import type { RepoEntry } from "../lib/bindings";

interface Props {
  /// "tile" = sidebar tile under the drawer list. "primary" = onboarding-style
  /// primary button. They differ only in styling.
  variant?: "tile" | "primary";
  onAdded?: (repo: RepoEntry) => void;
}

export function AddRepoButton({ variant = "tile", onAdded }: Props) {
  const pastePathEnabled = settings.value.pastePathEnabled;
  const [pasting, setPasting] = useState(false);
  const [pastePath, setPastePath] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (path: string) => {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      const repo = await addRepoByPath(path);
      setPasting(false);
      setPastePath("");
      onAdded?.(repo);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const onPickFolder = async () => {
    if (busy) return;
    try {
      const selected = await open({ directory: true, multiple: false });
      if (typeof selected === "string") {
        await submit(selected);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div class={`add-repo add-repo-${variant}`}>
      <button
        type="button"
        class="add-repo-pick"
        disabled={busy}
        onClick={() => void onPickFolder()}
      >
        <span class="add-repo-glyph">＋</span>
        <span>{variant === "primary" ? "Choose folder…" : "Add repo…"}</span>
      </button>

      {pastePathEnabled &&
        (pasting ? (
          <div class="add-repo-paste">
            <input
              type="text"
              class="add-repo-input"
              placeholder="/absolute/path/to/repo"
              value={pastePath}
              onInput={(e) => setPastePath((e.target as HTMLInputElement).value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && pastePath.trim()) {
                  void submit(pastePath.trim());
                }
                if (e.key === "Escape") {
                  setPasting(false);
                  setPastePath("");
                }
              }}
              autoFocus
            />
            <button
              type="button"
              class="add-repo-submit"
              disabled={busy || !pastePath.trim()}
              onClick={() => void submit(pastePath.trim())}
            >
              Add
            </button>
          </div>
        ) : (
          <button type="button" class="add-repo-paste-toggle" onClick={() => setPasting(true)}>
            Paste path
          </button>
        ))}

      {error && <p class="add-repo-error">{error}</p>}
    </div>
  );
}
