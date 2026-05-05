import { useEffect, useState } from "preact/hooks";
import { commands } from "../../lib/bindings";
import type { IssueProvider } from "../../lib/bindings";

type ProviderKind = IssueProvider["kind"];

interface Props {
  repoName: string;
  kind: ProviderKind;
}

function statusLabel(exists: boolean | null): string {
  if (exists === null) return "…";
  if (exists) return "✓ Token saved in Keychain";
  return "No token saved";
}

/// Save / clear the macOS-Keychain-backed token for one (repo, provider)
/// pair. The token itself is never returned over IPC — `setProviderSecret`
/// is write-only and `providerSecretExists` is the only readback.
export function ProviderTokenControls({ repoName, kind }: Props) {
  const [exists, setExists] = useState<boolean | null>(null);
  const [draft, setDraft] = useState("");
  const [saving, setSaving] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);

  useEffect(() => {
    setExists(null);
    setMsg(null);
    void (async () => {
      const r = await commands.providerSecretExists(repoName, kind);
      setExists(r.status === "ok" ? r.data : false);
    })();
  }, [repoName, kind]);

  const onSave = async () => {
    if (!draft || saving) return;
    setSaving(true);
    const r = await commands.setProviderSecret(repoName, kind, draft);
    setSaving(false);
    if (r.status === "error") {
      setMsg(`Save failed: ${r.error}`);
      return;
    }
    setExists(true);
    setDraft("");
    setMsg("Token saved to Keychain.");
  };

  const onClear = async () => {
    const r = await commands.deleteProviderSecret(repoName, kind);
    if (r.status === "error") {
      setMsg(`Delete failed: ${r.error}`);
      return;
    }
    setExists(false);
    setMsg("Token cleared.");
  };

  return (
    <div class="repo-provider-token">
      <div class="repo-provider-token-status">{statusLabel(exists)}</div>
      <div class="repo-provider-token-row">
        <input
          type="password"
          class="repo-provider-input"
          placeholder="Paste API token"
          value={draft}
          onInput={(e) => setDraft((e.target as HTMLInputElement).value)}
        />
        <button
          type="button"
          class="prompt-btn"
          disabled={!draft || saving}
          onClick={() => void onSave()}
        >
          {saving ? "Saving…" : "Save token"}
        </button>
        {exists && (
          <button type="button" class="prompt-btn" onClick={() => void onClear()}>
            Clear
          </button>
        )}
      </div>
      {msg && <p class="prompt-status">{msg}</p>}
    </div>
  );
}
