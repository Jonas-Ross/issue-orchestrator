import { signal } from "@preact/signals";

const KEY = "io.sidebar.collapsed";

function loadInitial(storage: Storage): boolean {
  try {
    return storage.getItem(KEY) === "1";
  } catch {
    return false;
  }
}

export function createSidebarStore(storage: Storage = localStorage) {
  const sidebarCollapsed = signal(loadInitial(storage));
  function toggleSidebar() {
    const next = !sidebarCollapsed.value;
    sidebarCollapsed.value = next;
    try {
      storage.setItem(KEY, next ? "1" : "0");
    } catch {
      /* private mode: skip */
    }
  }
  return { sidebarCollapsed, toggleSidebar };
}

export const sidebarStore = createSidebarStore();
export const { sidebarCollapsed, toggleSidebar } = sidebarStore;
