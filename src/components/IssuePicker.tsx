import { useEffect, useMemo, useRef, useState } from "preact/hooks";
import { open } from "@tauri-apps/plugin-shell";
import { commands } from "../lib/bindings";
import type { Decision, Issue, RepoEntry } from "../lib/bindings";
import { activeId } from "../state/sessions";
import { closePicker, pickerOpen } from "../state/picker";
import { openContextMenu } from "../state/context-menu";

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
  const [search, setSearch] = useState("");
  const [activeLabels, setActiveLabels] = useState<Set<string>>(new Set());
  const [expanded, setExpanded] = useState<number | null>(null);
  const [bodies, setBodies] = useState<Map<number, string | "loading" | { error: string }>>(
    new Map(),
  );
  const [recommendation, setRecommendation] = useState<Decision | null>(null);
  const [recommending, setRecommending] = useState(false);
  const [recoError, setRecoError] = useState<string | null>(null);
  const listRef = useRef<HTMLUListElement | null>(null);

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
    setRecommendation(null);
    setRecoError(null);
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

  const allIssues = issues.tag === "ok" ? issues.issues : [];

  const allLabels = useMemo(() => {
    const set = new Set<string>();
    for (const i of allIssues) for (const l of i.labels) set.add(l);
    return Array.from(set).sort();
  }, [allIssues]);

  const filteredIssues = useMemo(() => {
    const q = search.trim().toLowerCase();
    return allIssues.filter((i) => {
      if (q) {
        const hay = `#${i.number} ${i.title}`.toLowerCase();
        if (!hay.includes(q)) return false;
      }
      for (const l of activeLabels) {
        if (!i.labels.includes(l)) return false;
      }
      return true;
    });
  }, [allIssues, search, activeLabels]);

  useEffect(() => {
    if (!recommendation || !listRef.current) return;
    const el = listRef.current.querySelector(
      `[data-issue-number="${recommendation.number}"]`,
    );
    if (el && "scrollIntoView" in el) {
      (el as HTMLElement).scrollIntoView({ block: "nearest", behavior: "smooth" });
    }
  }, [recommendation, filteredIssues]);

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

  const onDecide = async () => {
    if (!selectedRepo || recommending) return;
    setRecommending(true);
    setRecoError(null);
    const result = await commands.decideNextIssue(selectedRepo);
    setRecommending(false);
    if (result.status === "error") {
      setRecoError(result.error);
      return;
    }
    setRecommendation(result.data);
  };

  const toggleLabel = (label: string) => {
    setActiveLabels((prev) => {
      const next = new Set(prev);
      if (next.has(label)) next.delete(label);
      else next.add(label);
      return next;
    });
  };

  const toggleExpand = async (issue: Issue) => {
    if (expanded === issue.number) {
      setExpanded(null);
      return;
    }
    setExpanded(issue.number);
    if (!selectedRepo) return;
    if (bodies.has(issue.number)) return;
    setBodies((prev) => new Map(prev).set(issue.number, "loading"));
    const result = await commands.getIssueBody(selectedRepo, issue.number);
    setBodies((prev) => {
      const next = new Map(prev);
      next.set(
        issue.number,
        result.status === "error" ? { error: result.error } : result.data,
      );
      return next;
    });
  };

  const onIssueContextMenu = (e: MouseEvent, issue: Issue) => {
    e.preventDefault();
    e.stopPropagation();
    openContextMenu({
      x: e.clientX,
      y: e.clientY,
      items: [
        { label: "Open issue ↗", action: () => void open(issue.url) },
        {
          label: "Copy issue link",
          action: () => void navigator.clipboard.writeText(issue.url),
        },
        {
          label: "Copy branch name",
          action: () => void navigator.clipboard.writeText(`issue-${issue.number}`),
        },
      ],
    });
  };

  return (
    <div class="modal-overlay" onClick={() => closePicker()}>
      <div class="modal" onClick={(e) => e.stopPropagation()}>
        <div class="modal-header">
          <h2>Pick an issue</h2>
          <div class="modal-header-actions">
            <button
              type="button"
              class="decide-btn"
              disabled={!selectedRepo || recommending || allIssues.length === 0}
              title="Ask Claude to recommend the best next issue"
              onClick={() => void onDecide()}
            >
              {recommending ? "Thinking…" : "Suggest a task"}
            </button>
            <button type="button" class="close" onClick={() => closePicker()}>
              ×
            </button>
          </div>
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
        {selectedRepo && issues.tag === "ok" && issues.issues.length > 0 && (
          <div class="picker-toolbar">
            <input
              type="text"
              class="issue-search"
              placeholder="Search by title or #number"
              value={search}
              onInput={(e) => setSearch((e.target as HTMLInputElement).value)}
            />
            {allLabels.length > 0 && (
              <div class="label-chips">
                {allLabels.map((l) => (
                  <span
                    key={l}
                    class={`chip${activeLabels.has(l) ? " active" : ""}`}
                    onClick={() => toggleLabel(l)}
                  >
                    {l}
                  </span>
                ))}
              </div>
            )}
            {recommendation && (
              <p class="hint" style={{ margin: 0 }}>
                AI recommends <strong>#{recommendation.number}</strong> — {recommendation.reasoning}
              </p>
            )}
            {recoError && <p class="error" style={{ margin: 0, padding: 0 }}>Decide failed: {recoError}</p>}
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
          <ul class="issue-list" ref={listRef}>
            {filteredIssues.length === 0 && (
              <li class="hint" style={{ padding: "12px 16px", borderBottom: "none" }}>
                No matching issues.
              </li>
            )}
            {filteredIssues.map((issue) => {
              const isAiPick = recommendation?.number === issue.number;
              const isExpanded = expanded === issue.number;
              const body = bodies.get(issue.number);
              return (
                <li
                  key={issue.number}
                  class={
                    "issue" +
                    (spawning === issue.number ? " spawning" : "") +
                    (isAiPick ? " ai-pick" : "")
                  }
                  data-issue-number={issue.number}
                  style={{ gridTemplateColumns: "auto auto 1fr auto auto" }}
                  onClick={() => void onSpawn(issue)}
                  onContextMenu={(e) => onIssueContextMenu(e, issue)}
                >
                  <button
                    type="button"
                    class="issue-caret"
                    title={isExpanded ? "Collapse" : "Show body"}
                    onClick={(e) => {
                      e.stopPropagation();
                      void toggleExpand(issue);
                    }}
                  >
                    {isExpanded ? "▾" : "▸"}
                  </button>
                  <a
                    class="issue-number"
                    href={issue.url}
                    title="Open on GitHub"
                    onClick={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      void open(issue.url);
                    }}
                  >
                    #{issue.number}
                  </a>
                  <span class="issue-title">{issue.title}</span>
                  <span class="issue-labels">
                    {isAiPick && <span class="label ai-pick">AI pick</span>}
                    {issue.labels.map((l) => (
                      <span class="label">{l}</span>
                    ))}
                  </span>
                  {spawning === issue.number && (
                    <span class="spinner">Spawning…</span>
                  )}
                  {isAiPick && recommendation && (
                    <span class="issue-reasoning">{recommendation.reasoning}</span>
                  )}
                  {isExpanded && (
                    <pre class="issue-body">
                      {body === undefined || body === "loading"
                        ? "Loading…"
                        : typeof body === "string"
                          ? body || "(no body)"
                          : `Failed to load body: ${body.error}`}
                    </pre>
                  )}
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </div>
  );
}
