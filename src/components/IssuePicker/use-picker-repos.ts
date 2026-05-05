import { useEffect, useState } from "preact/hooks";
import { loadRepos, repos as reposSignal } from "../../state/repos";

export function usePickerRepos(prefilledRepo: string | null) {
  const repos = reposSignal.value;
  const [reposError, setReposError] = useState<string | null>(null);
  const [selectedRepo, setSelectedRepo] = useState<string | null>(prefilledRepo);

  // Drawer-launched picker has the repo fixed; refresh on every open in
  // case the user added/removed a repo with the picker closed.
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

  return { repos, reposError, selectedRepo, setSelectedRepo };
}
