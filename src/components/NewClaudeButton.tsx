import { spawnClaude } from "../lib/spawn-claude";

interface Props {
  className: string;
  /** Pass a repo name to bucket the session under that drawer; omit for footer scratch. */
  repoName?: string;
  /** Drawer headers double as the toggle row, so the button stops propagation. */
  stopPropagation?: boolean;
}

export function NewClaudeButton({ className, repoName, stopPropagation }: Props) {
  const title = repoName ? `New Claude session in ${repoName}` : "New Claude session (scratch)";
  return (
    <button
      type="button"
      class={className}
      title={title}
      onClick={(e) => {
        if (stopPropagation) e.stopPropagation();
        void spawnClaude(repoName);
      }}
    >
      ✦
    </button>
  );
}
