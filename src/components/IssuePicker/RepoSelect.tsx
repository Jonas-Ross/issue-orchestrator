interface Props {
  repos: { name: string }[];
  selectedRepo: string | null;
  onChange: (name: string) => void;
  refAttach: (el: HTMLSelectElement | null) => void;
}

export function RepoSelect({ repos, selectedRepo, onChange, refAttach }: Props) {
  return (
    <div class="row">
      <label>
        Repo:{" "}
        <select
          ref={refAttach}
          value={selectedRepo ?? ""}
          onChange={(e) => onChange((e.target as HTMLSelectElement).value)}
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
  );
}
