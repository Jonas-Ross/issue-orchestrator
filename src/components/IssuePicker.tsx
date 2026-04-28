import { useEffect, useMemo, useRef, useState } from "preact/hooks";
import { open } from "@tauri-apps/plugin-shell";
import { commands } from "../lib/bindings";
import type { Decision, Issue } from "../lib/bindings";
import { useFocusRestore, useFocusTrap } from "../lib/use-focus-trap";
import { activeId } from "../state/sessions";
import { closePicker, pickerOpen } from "../state/picker";
import { openContextMenu } from "../state/context-menu";
import { loadRepos, repos as reposSignal } from "../state/repos";

type IssueState =
  | { tag: "idle" }
  | { tag: "loading" }
  | { tag: "ok"; issues: Issue[] }
  | { tag: "error"; message: string };

/// Thin wrapper that mounts/unmounts the inner picker around the open
/// signal. Keeping the hook-bearing code in `IssuePickerInner` means
/// every open is a fresh mount (so `useState(prefilledRepo)` actually
/// honors the prefill, and effects re-register cleanly).
export function IssuePicker() {
  const state = pickerOpen.value;
  if (!state) return null;
  return <IssuePickerInner prefilledRepo={state.repoName} />;
}

function IssuePickerInner({ prefilledRepo }: { prefilledRepo: string | null }) {
  const repos = reposSignal.value;
  const [reposError, setReposError] = useState<string | null>(null);
  const [selectedRepo, setSelectedRepo] = useState<string | null>(prefilledRepo);
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
  const [highlightedIndex, setHighlightedIndex] = useState(0);
  const listRef = useRef<HTMLUListElement | null>(null);
  const modalRef = useRef<HTMLDivElement | null>(null);
  const focusedOnce = useRef(false);

  // Default focus: whichever priority element mounts first wins. The repo
  // dropdown renders as soon as listRepos resolves; the search input
  // appears after listIssues. Without a prefill the dropdown beats the
  // search; with a prefill (drawer-launched) the search wins by default.
  const onSelectRefAttach = (el: HTMLSelectElement | null) => {
    if (el && !focusedOnce.current) {
      el.focus();
      focusedOnce.current = true;
    }
  };
  const onSearchRefAttach = (el: HTMLInputElement | null) => {
    if (el && !focusedOnce.current) {
      el.focus();
      focusedOnce.current = true;
    }
  };

  // Park focus on the modal as a fallback only — ref-attach handlers
  // (the select dropdown / search input) run during commit and may have
  // already claimed focus before this effect fires. Stealing it back
  // would defeat the priority-focus logic.
  useEffect(() => {
    if (!focusedOnce.current) {
      modalRef.current?.focus();
    }
  }, []);
  useFocusRestore();
  useFocusTrap(modalRef);

  // Drawer-launched picker has the repo fixed; the global signal is
  // already loaded at app boot, so we just refresh on every open in case
  // the user added/removed a repo with the picker closed.
  useEffect(() => {
    if (prefilledRepo) return;
    loadRepos().catch((e) => setReposError(String(e)));
  }, [prefilledRepo]);

  // Auto-select the only repo so the picker skips straight to issues.
  useEffect(() => {
    if (prefilledRepo) return;
    if (repos.length === 1 && selectedRepo === null) {
      setSelectedRepo(repos[0].name);
    }
  }, [repos, prefilledRepo, selectedRepo]);

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

  // Clamp highlight when the filtered set shrinks (search typing). Reads
  // `highlightedIndex` via the functional setter so it doesn't need to be
  // a dependency — otherwise this effect would re-fire on every arrow key.
  useEffect(() => {
    setHighlightedIndex((i) => {
      if (filteredIssues.length === 0) return 0;
      if (i >= filteredIssues.length) return filteredIssues.length - 1;
      return i;
    });
  }, [filteredIssues]);

  // Auto-scroll the highlighted issue into view as the user arrows.
  useEffect(() => {
    if (filteredIssues.length === 0 || !listRef.current) return;
    const target = filteredIssues[highlightedIndex];
    if (!target) return;
    const el = listRef.current.querySelector(
      `[data-issue-number="${target.number}"]`,
    );
    if (el && "scrollIntoView" in el) {
      (el as HTMLElement).scrollIntoView({ block: "nearest" });
    }
  }, [highlightedIndex, filteredIssues]);

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

  // Arrow-key nav over filteredIssues + Enter to spawn. Capture phase so
  // we beat the global keymap. Modifier-key combos pass through so
  // ⌘N / ⌘W / etc. still work over the picker.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      if (e.key === "ArrowDown") {
        if (filteredIssues.length === 0) return;
        e.preventDefault();
        e.stopPropagation();
        setHighlightedIndex((i) => (i + 1) % filteredIssues.length);
      } else if (e.key === "ArrowUp") {
        if (filteredIssues.length === 0) return;
        e.preventDefault();
        e.stopPropagation();
        setHighlightedIndex(
          (i) => (i - 1 + filteredIssues.length) % filteredIssues.length,
        );
      } else if (e.key === "Enter") {
        // Don't hijack Enter inside the search input — that would feel
        // broken if the user is mid-type. Only trigger when focus is
        // outside an input, OR explicitly on the issue list.
        const tgt = e.target as HTMLElement | null;
        const inInput = tgt?.tagName === "INPUT" || tgt?.tagName === "TEXTAREA";
        if (inInput && tgt?.classList.contains("issue-search") === false) {
          return;
        }
        const target = filteredIssues[highlightedIndex];
        if (!target) return;
        e.preventDefault();
        e.stopPropagation();
        void onSpawn(target);
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [filteredIssues, highlightedIndex, selectedRepo, spawning]);

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
      <div
        class="modal"
        ref={modalRef}
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
      >
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
              <span style={{ fontFamily: "var(--font-mono)", fontSize: 10 }}>✦</span>
              {recommending ? "Thinking…" : "Suggest a task"}
            </button>
            <button type="button" class="close" onClick={() => closePicker()}>
              ×
            </button>
          </div>
        </div>
        {!prefilledRepo && reposError && (
          <p class="error">Failed to load repos: {reposError}</p>
        )}
        {!prefilledRepo && !reposError && repos.length === 0 && (
          <p class="hint">
            No repos configured. Use the sidebar's "+ Add repo…" to register one.
          </p>
        )}
        {!prefilledRepo && repos.length > 1 && (
          <div class="row">
            <label>
              Repo:{" "}
              <select
                ref={onSelectRefAttach}
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
              ref={onSearchRefAttach}
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
            {filteredIssues.map((issue, idx) => {
              const isAiPick = recommendation?.number === issue.number;
              const isExpanded = expanded === issue.number;
              const isHighlighted = idx === highlightedIndex;
              const body = bodies.get(issue.number);
              return (
                <li
                  key={issue.number}
                  class={
                    "issue" +
                    (spawning === issue.number ? " spawning" : "") +
                    (isAiPick ? " ai-pick" : "") +
                    (isHighlighted ? " highlighted" : "")
                  }
                  data-issue-number={issue.number}
                  style={{ gridTemplateColumns: "auto auto 1fr auto auto" }}
                  onClick={() => void onSpawn(issue)}
                  onContextMenu={(e) => onIssueContextMenu(e, issue)}
                  onMouseEnter={() => setHighlightedIndex(idx)}
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
