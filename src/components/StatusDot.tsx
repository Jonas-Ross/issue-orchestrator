import type { Status } from "../lib/bindings";

interface Props {
  status: Status;
  size?: number;
  pulse?: boolean;
}

export function StatusDot({ status, size = 8, pulse = false }: Props) {
  const cls = `status-dot status-dot-${status}` + (pulse ? " pulse" : "");
  return (
    <span class={cls} style={{ width: `${size}px`, height: `${size}px` }} aria-label={status}>
      {pulse && <span class="status-dot-ring" />}
    </span>
  );
}
