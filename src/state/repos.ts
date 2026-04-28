import { signal } from "@preact/signals";
import { commands } from "../lib/bindings";
import type { RepoEntry } from "../lib/bindings";

const STORAGE_KEY = "io.drawers.expanded.v1";

export const repos = signal<RepoEntry[]>([]);

/// Per-repo drawer expansion state, mirrored to localStorage so it
/// survives reloads. Keyed by repo name. Missing entries default to
/// expanded (true) on first render.
export const drawerExpanded = signal<Record<string, boolean>>(loadDrawerState());

function loadDrawerState(): Record<string, boolean> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    return typeof parsed === "object" && parsed !== null ? parsed : {};
  } catch {
    return {};
  }
}

function persistDrawerState() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(drawerExpanded.value));
  } catch {
    // best effort; storage may be disabled
  }
}

export function isDrawerExpanded(name: string): boolean {
  const v = drawerExpanded.value[name];
  return v === undefined ? true : v;
}

export function toggleDrawer(name: string) {
  const current = isDrawerExpanded(name);
  drawerExpanded.value = { ...drawerExpanded.value, [name]: !current };
  persistDrawerState();
}

export async function loadRepos(): Promise<void> {
  const result = await commands.listRepos();
  if (result.status === "ok") {
    repos.value = result.data;
  } else {
    console.error("listRepos failed:", result.error);
  }
}

export async function addRepoByPath(path: string): Promise<RepoEntry> {
  const result = await commands.addRepo(path);
  if (result.status === "error") {
    throw new Error(result.error);
  }
  await loadRepos();
  return result.data;
}

export async function removeRepoByName(name: string): Promise<void> {
  const result = await commands.removeRepo(name);
  if (result.status === "error") {
    throw new Error(result.error);
  }
  await loadRepos();
}
