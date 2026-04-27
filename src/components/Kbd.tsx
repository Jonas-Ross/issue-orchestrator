import type { ComponentChildren } from "preact";

export function Kbd({ children }: { children: ComponentChildren }) {
  return <span class="kbd">{children}</span>;
}
