interface Props {
  prefilledRepo: string | null;
  reposError: string | null;
  repoCount: number;
}

export function RepoStatusBanner({ prefilledRepo, reposError, repoCount }: Props) {
  if (prefilledRepo) return null;
  if (reposError) return <p class="error">Failed to load repos: {reposError}</p>;
  if (repoCount === 0) {
    return (
      <p class="hint">No repos configured. Use the sidebar's "+ Add repo…" to register one.</p>
    );
  }
  return null;
}
