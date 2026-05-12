import { open } from "@tauri-apps/plugin-shell";
import type { ChecksRollup, PrStatus } from "../lib/bindings";

interface Props {
  prStatus: PrStatus;
}

const CI_LABEL: Record<ChecksRollup, string> = {
  pass: "CI passed",
  fail: "CI failed",
  pending: "CI running",
  none: "No CI checks",
};

export function PrChip({ prStatus }: Props) {
  const onClick = (e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    void open(prStatus.url);
  };

  return (
    <button
      type="button"
      class={`pr-chip pr-chip-${prStatus.checks}`}
      title={`PR #${prStatus.number} — ${CI_LABEL[prStatus.checks]}`}
      onClick={onClick}
    >
      <span class="pr-chip-dot" aria-hidden="true" />
      <span class="pr-chip-label">PR #{prStatus.number}</span>
    </button>
  );
}
