import { useCallback, useMemo, useRef, useState } from "preact/hooks";
import { commands } from "../../lib/bindings";
import { DEFAULT_PTY_COLS, DEFAULT_PTY_ROWS } from "../../lib/constants";
import { activeId } from "../../state/sessions";
import { closePicker, pickerOpen } from "../../state/picker";
import { Modal } from "../Modal";
import { IssueList } from "./IssueList";
import { IssuePickerHeader } from "./IssuePickerHeader";
import { IssuePickerToolbar } from "./IssuePickerToolbar";
import { IssuePromptPreview } from "./IssuePromptPreview";
import { IssuesStatusBanner } from "./IssuesStatusBanner";
import { RepoSelect } from "./RepoSelect";
import { RepoStatusBanner } from "./RepoStatusBanner";
import type { Issue } from "./types";
import { useHighlightedIssue } from "./use-highlighted-issue";
import { useIssueBodies } from "./use-issue-bodies";
import { useIssueRecommendation } from "./use-issue-recommendation";
import { useIssuesList } from "./use-issues-list";
import { usePickerKeyboard } from "./use-picker-keyboard";
import { usePickerRepos } from "./use-picker-repos";
import { usePriorityFocus } from "./use-priority-focus";
import { usePromptDraft } from "./use-prompt-draft";

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
  const { repos, reposError, selectedRepo, setSelectedRepo } = usePickerRepos(prefilledRepo);
  const [issues, setIssues] = useIssuesList(selectedRepo);
  const [spawning, setSpawning] = useState<number | null>(null);
  const [search, setSearch] = useState("");
  const [activeLabels, setActiveLabels] = useState<Set<string>>(new Set());
  const listRef = useRef<HTMLUListElement | null>(null);
  const { modalRef, selectRefAttach, searchRefAttach } = usePriorityFocus();

  const allIssues = useMemo(() => (issues.tag === "ok" ? issues.issues : []), [issues]);
  const allLabels = useMemo(() => {
    const set = new Set<string>();
    for (const i of allIssues) for (const l of i.labels) set.add(l);
    return Array.from(set).sort();
  }, [allIssues]);
  const filteredIssues = useMemo(() => {
    const q = search.trim().toLowerCase();
    return allIssues.filter((i) => {
      if (q && !`#${i.number} ${i.title}`.toLowerCase().includes(q)) return false;
      for (const l of activeLabels) if (!i.labels.includes(l)) return false;
      return true;
    });
  }, [allIssues, search, activeLabels]);

  const { expanded, bodies, toggleExpand } = useIssueBodies(selectedRepo);
  const { recommendation, recommending, recoError, onDecide } = useIssueRecommendation(
    selectedRepo,
    filteredIssues,
    listRef,
  );
  const [highlightedIndex, setHighlightedIndex] = useHighlightedIssue(filteredIssues, listRef);
  const highlightedIssue =
    highlightedIndex !== null ? (filteredIssues[highlightedIndex] ?? null) : null;
  const promptDraft = usePromptDraft(highlightedIssue);

  const onSpawn = useCallback(
    async (issue: Issue) => {
      if (!selectedRepo || spawning !== null) return;
      setSpawning(issue.number);
      // Pass the rendered override only when this issue has one — otherwise
      // the backend resolves saved-template → default on its own.
      const result = await commands.spawnIssueSession(
        selectedRepo,
        issue.number,
        DEFAULT_PTY_COLS,
        DEFAULT_PTY_ROWS,
        promptDraft.getOverrideFor(issue),
      );
      setSpawning(null);
      if (result.status === "error") {
        setIssues({ tag: "error", message: result.error });
        return;
      }
      activeId.value = result.data.id;
      closePicker();
    },
    [selectedRepo, spawning, setIssues, promptDraft],
  );

  usePickerKeyboard({
    filteredIssues,
    highlightedIndex,
    setHighlightedIndex,
    onSpawn: (issue) => void onSpawn(issue),
  });

  const toggleLabel = (label: string) => {
    setActiveLabels((prev) => {
      const next = new Set(prev);
      if (next.has(label)) next.delete(label);
      else next.add(label);
      return next;
    });
  };

  const hasIssues = issues.tag === "ok" && issues.issues.length > 0;
  const showRepoSelect = !prefilledRepo && repos.length > 1;

  return (
    <Modal onClose={() => closePicker()} dialogRef={modalRef} tabIndex={-1}>
      <IssuePickerHeader
        canDecide={!!selectedRepo && !recommending && allIssues.length > 0}
        recommending={recommending}
        onDecide={() => void onDecide()}
        onClose={() => closePicker()}
      />
      <RepoStatusBanner
        prefilledRepo={prefilledRepo}
        reposError={reposError}
        repoCount={repos.length}
      />
      {showRepoSelect && (
        <RepoSelect
          repos={repos}
          selectedRepo={selectedRepo}
          onChange={setSelectedRepo}
          refAttach={selectRefAttach}
        />
      )}
      {selectedRepo && hasIssues && (
        <IssuePickerToolbar
          search={search}
          setSearch={setSearch}
          allLabels={allLabels}
          activeLabels={activeLabels}
          toggleLabel={toggleLabel}
          recommendation={recommendation}
          recoError={recoError}
          searchRefAttach={searchRefAttach}
        />
      )}
      <IssuesStatusBanner selectedRepo={selectedRepo} issues={issues} />
      {selectedRepo && hasIssues && (
        <IssuePromptPreview issue={highlightedIssue} draft={promptDraft} />
      )}
      {selectedRepo && hasIssues && (
        <IssueList
          listRef={listRef}
          issues={filteredIssues}
          highlightedIndex={highlightedIndex}
          expanded={expanded}
          recommendation={recommendation}
          spawning={spawning}
          bodies={bodies}
          onSpawn={(i) => void onSpawn(i)}
          onToggleExpand={(i) => void toggleExpand(i)}
          onHighlight={setHighlightedIndex}
        />
      )}
    </Modal>
  );
}
