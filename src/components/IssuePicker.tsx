import { useEffect, useState } from "preact/hooks";
import { commands } from "../lib/bindings";
import type { Issue, RepoEntry } from "../lib/bindings";
import { activeId } from "../state/sessions";
import { closePicker, pickerOpen } from "../state/picker";

type IssueState =
  | { tag: "idle" }
  | { tag: "loading" }
  | { tag: "ok"; issues: Issue[] }
  | { tag: "error"; message: string };

export function IssuePicker() {
  if (!pickerOpen.value) return null;

  const [repos, setRepos] = useState<RepoEntry[] | null>(null);
  const [reposError, setReposError] = useState<string | null>(null);
  const [selectedRepo, setSelectedRepo] = useState<string | null>(null);
  const [issues, setIssues] = useState<IssueState>({ tag: "idle" });
  const [spawning, setSpawning] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const result = await commands.listRepos();
      if (cancelled) return;
      if (result.status === "error") {
        setReposError(result.error);
        return;
      }
      setRepos(result.data);
      if (result.data.length === 1) setSelectedRepo(result.data[0].name);
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!selectedRepo) return;
    let cancelled = false;
    setIssues({ tag: "loading" });
    void (async () => {
      const result = await commands.listIssues(selectedRepo);
      if (cancelled) return;
      if (result.status === "error") {
        setIssues({ tag: "error", message: result.error });
        return;
      }
      setIssues({ tag: "ok", issues: result.data });
    })();
    return () => {
      cancelled = true;
    };
  }, [selectedRepo]);

  const onSpawn = async (issue: Issue) => {
    if (!selectedRepo || spawning !== null) return;
    setSpawning(issue.number);
    const result = await commands.spawnIssueSession(
      selectedRepo,
      issue.number,
      80,
      24,
    );
    setSpawning(null);
    if (result.status === "error") {
      setIssues({ tag: "error", message: result.error });
      return;
    }
    activeId.value = result.data.id;
    closePicker();
  };

  return (
    <div class="modal-overlay" onClick={() => closePicker()}>
      <div class="modal" onClick={(e) => e.stopPropagation()}>
        <div class="modal-header">
          <h2>Pick an issue</h2>
          <button type="button" class="close" onClick={() => closePicker()}>
            ×
          </button>
        </div>
        {reposError && <p class="error">Failed to load repos: {reposError}</p>}
        {!reposError && repos && repos.length === 0 && (
          <p class="hint">
            No repos configured. Add a <code>repos</code> entry to your
            config.json to enable the picker.
          </p>
        )}
        {repos && repos.length > 1 && (
          <div class="row">
            <label>
              Repo:{" "}
              <select
                value={selectedRepo ?? ""}
                onChange={(e) =>
                  setSelectedRepo((e.target as HTMLSelectElement).value)
                }
              >
                <option value="" disabled>
                  Select a repo
                </option>
                {repos.map((r) => (
                  <option key={r.name} value={r.name}>
                    {r.name}
                  </option>
                ))}
              </select>
            </label>
          </div>
        )}
        {selectedRepo && issues.tag === "loading" && (
          <p class="hint">Loading issues…</p>
        )}
        {selectedRepo && issues.tag === "error" && (
          <p class="error">Failed to load issues: {issues.message}</p>
        )}
        {selectedRepo && issues.tag === "ok" && issues.issues.length === 0 && (
          <p class="hint">No open issues.</p>
        )}
        {selectedRepo && issues.tag === "ok" && issues.issues.length > 0 && (
          <ul class="issue-list">
            {issues.issues.map((issue) => (
              <li
                key={issue.number}
                class={`issue${spawning === issue.number ? " spawning" : ""}`}
                onClick={() => void onSpawn(issue)}
              >
                <span class="issue-number">#{issue.number}</span>
                <span class="issue-title">{issue.title}</span>
                <span class="issue-labels">
                  {issue.labels.map((l) => (
                    <span class="label">{l}</span>
                  ))}
                </span>
                {spawning === issue.number && (
                  <span class="spinner">Spawning…</span>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
