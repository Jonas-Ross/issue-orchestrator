import { useEffect, useState } from "preact/hooks";
import { commands } from "../../lib/bindings";
import { DEFAULT_SPAWN_PROMPT } from "../../lib/spawn-prompt";
import { repos } from "../../state/repos";

/// Settings panel category that owns the saved spawn prompt template.
/// State is local because no other surface needs to read it — the
/// IssuePicker resolves its own copy via `commands.getConfig()` when it
/// opens, so there's no shared signal to keep in sync.
export function PromptSection() {
  const [draft, setDraft] = useState<string>("");
  const [savedTemplate, setSavedTemplate] = useState<string | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [saving, setSaving] = useState(false);
  const [optimizingRepo, setOptimizingRepo] = useState<string | null>(null);
  const [optimizeError, setOptimizeError] = useState<string | null>(null);
  const [statusMsg, setStatusMsg] = useState<string | null>(null);

  const repoList = repos.value;
  const [selectedRepo, setSelectedRepo] = useState<string | null>(repoList[0]?.name ?? null);

  useEffect(() => {
    void (async () => {
      const result = await commands.getConfig();
      if (result.status === "ok") {
        const tpl = result.data.spawnPromptTemplate ?? null;
        setSavedTemplate(tpl);
        setDraft(tpl ?? DEFAULT_SPAWN_PROMPT);
      }
      setLoaded(true);
    })();
  }, []);

  // Keep the repo selector pinned to a real entry as the list changes.
  useEffect(() => {
    if (selectedRepo && repoList.some((r) => r.name === selectedRepo)) return;
    setSelectedRepo(repoList[0]?.name ?? null);
  }, [repoList, selectedRepo]);

  const isDirty = draft !== (savedTemplate ?? DEFAULT_SPAWN_PROMPT);

  const onSave = async () => {
    if (saving) return;
    setSaving(true);
    setStatusMsg(null);
    const trimmed = draft.trim();
    // Empty draft → clear back to "use built-in default"; otherwise persist
    // the new template. Backend already filters empties to None.
    const payload = trimmed.length === 0 ? null : draft;
    const result = await commands.updateSpawnPrompt(payload);
    setSaving(false);
    if (result.status === "error") {
      setStatusMsg(`Save failed: ${result.error}`);
      return;
    }
    setSavedTemplate(payload);
    if (payload === null) setDraft(DEFAULT_SPAWN_PROMPT);
    setStatusMsg("Saved.");
  };

  const onReset = async () => {
    setDraft(DEFAULT_SPAWN_PROMPT);
    if (savedTemplate !== null) {
      const result = await commands.updateSpawnPrompt(null);
      if (result.status === "error") {
        setStatusMsg(`Reset failed: ${result.error}`);
        return;
      }
      setSavedTemplate(null);
      setStatusMsg("Reset to default.");
    }
  };

  const onOptimize = async () => {
    if (!selectedRepo || optimizingRepo) return;
    setOptimizingRepo(selectedRepo);
    setOptimizeError(null);
    setStatusMsg(null);
    const result = await commands.optimizeSpawnPrompt(selectedRepo, draft);
    setOptimizingRepo(null);
    if (result.status === "error") {
      setOptimizeError(result.error);
      return;
    }
    setDraft(result.data);
  };

  if (!loaded) return <p class="settings-row-desc">Loading…</p>;

  const optimizing = optimizingRepo !== null;
  const canOptimize = !!selectedRepo && !optimizing;

  return (
    <div class="prompt-section">
      <p class="settings-row-desc">
        This template runs as the first message of every issue session. Use{" "}
        <code>{"{issue_number}"}</code> and <code>{"{issue_title}"}</code> as placeholders — they're
        substituted at spawn time.
      </p>
      <textarea
        class="prompt-textarea"
        rows={6}
        spellcheck={false}
        value={draft}
        onInput={(e) => setDraft((e.target as HTMLTextAreaElement).value)}
      />
      <div class="prompt-toolbar">
        <button
          type="button"
          class="prompt-btn primary"
          disabled={!isDirty || saving}
          onClick={() => void onSave()}
        >
          {saving ? "Saving…" : "Save"}
        </button>
        <button type="button" class="prompt-btn" onClick={() => void onReset()}>
          Reset to default
        </button>
        <div class="prompt-toolbar-right">
          {repoList.length > 1 && (
            <select
              class="prompt-repo-select"
              value={selectedRepo ?? ""}
              onChange={(e) => setSelectedRepo((e.target as HTMLSelectElement).value || null)}
            >
              {repoList.map((r) => (
                <option key={r.name} value={r.name}>
                  {r.name}
                </option>
              ))}
            </select>
          )}
          <button
            type="button"
            class="prompt-btn"
            disabled={!canOptimize}
            title={
              selectedRepo
                ? `Ask Claude in ${selectedRepo} to rewrite this`
                : "Add a repo to enable AI optimization"
            }
            onClick={() => void onOptimize()}
          >
            {optimizing ? "Optimizing…" : "Optimize with Claude"}
          </button>
        </div>
      </div>
      {optimizeError && <p class="prompt-error">Optimize failed: {optimizeError}</p>}
      {statusMsg && <p class="prompt-status">{statusMsg}</p>}
    </div>
  );
}
