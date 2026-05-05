import type { Status } from "../lib/bindings";

type Kind = "edit" | "wait" | "idle" | "exit" | "spawn" | "shell";

const ICONS: Record<Kind, preact.JSX.Element> = {
  edit: (
    <>
      <path
        d="M2 10L4 8L8 4L10 6L6 10L4 10L2 10Z"
        stroke="currentColor"
        stroke-width="1"
        stroke-linejoin="round"
        fill="none"
      />
      <path d="M7 5L9 7" stroke="currentColor" stroke-width="1" />
    </>
  ),
  wait: (
    <>
      <path
        d="M2 3.5C2 2.67 2.67 2 3.5 2H8.5C9.33 2 10 2.67 10 3.5V6.5C10 7.33 9.33 8 8.5 8H6L4 10V8H3.5C2.67 8 2 7.33 2 6.5V3.5Z"
        stroke="currentColor"
        stroke-width="1"
        stroke-linejoin="round"
        fill="none"
      />
      <circle cx="4.5" cy="5" r="0.5" fill="currentColor" />
      <circle cx="6" cy="5" r="0.5" fill="currentColor" />
      <circle cx="7.5" cy="5" r="0.5" fill="currentColor" />
    </>
  ),
  idle: (
    <path
      d="M9 6.5C8.5 8 7 9 5.5 8.8C4 8.7 3 7.5 3 6C3 4.5 4 3.3 5.5 3.2C5 4 5 5 5.5 5.8C6 6.5 7 7 8 7C8.4 7 8.7 6.8 9 6.5Z"
      stroke="currentColor"
      stroke-width="1"
      stroke-linejoin="round"
      fill="none"
    />
  ),
  exit: (
    <>
      <rect
        x="2"
        y="2"
        width="8"
        height="8"
        stroke="currentColor"
        stroke-width="1"
        rx="1"
        fill="none"
      />
      <path
        d="M4.5 4.5L7.5 7.5M7.5 4.5L4.5 7.5"
        stroke="currentColor"
        stroke-width="1"
        stroke-linecap="round"
      />
    </>
  ),
  spawn: (
    <circle
      cx="6"
      cy="6"
      r="4"
      stroke="currentColor"
      stroke-width="1"
      stroke-dasharray="1.5 1.5"
      fill="none"
    />
  ),
  shell: (
    <path
      d="M3 3L6 6L3 9M7 9H10"
      stroke="currentColor"
      stroke-width="1"
      stroke-linecap="round"
      stroke-linejoin="round"
      fill="none"
    />
  ),
};

export function ActivityIcon({ status, isShell = false }: { status: Status; isShell?: boolean }) {
  const kind: Kind = isShell
    ? "shell"
    : status === "needs_input"
      ? "wait"
      : status === "running"
        ? "edit"
        : status === "idle"
          ? "idle"
          : status === "exited"
            ? "exit"
            : "spawn";
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" class="activity-icon">
      {ICONS[kind]}
    </svg>
  );
}
