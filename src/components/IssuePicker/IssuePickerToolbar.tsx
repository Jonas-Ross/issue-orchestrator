import type { Decision } from "./types";

interface Props {
  search: string;
  setSearch: (s: string) => void;
  allLabels: string[];
  activeLabels: Set<string>;
  toggleLabel: (label: string) => void;
  recommendation: Decision | null;
  recoError: string | null;
  searchRefAttach: (el: HTMLInputElement | null) => void;
}

export function IssuePickerToolbar({
  search,
  setSearch,
  allLabels,
  activeLabels,
  toggleLabel,
  recommendation,
  recoError,
  searchRefAttach,
}: Props) {
  return (
    <div class="picker-toolbar">
      <input
        ref={searchRefAttach}
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
      {recoError && (
        <p class="error" style={{ margin: 0, padding: 0 }}>
          Decide failed: {recoError}
        </p>
      )}
    </div>
  );
}
