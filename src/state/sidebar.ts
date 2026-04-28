import { signal } from "@preact/signals";

const KEY = "io.sidebar.collapsed";

const initial = (() => {
  try {
    return localStorage.getItem(KEY) === "1";
  } catch {
    return false;
  }
})();

export const sidebarCollapsed = signal(initial);

export function toggleSidebar() {
  const next = !sidebarCollapsed.value;
  sidebarCollapsed.value = next;
  try {
    localStorage.setItem(KEY, next ? "1" : "0");
  } catch {
    /* private mode: skip */
  }
}
