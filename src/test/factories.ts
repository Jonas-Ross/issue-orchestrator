/// One-import test convenience: re-exports every state-module factory so
/// tests can do `import { createSessionsState } from "../../test/factories"`
/// instead of reaching into individual modules.

import type { SessionSummary } from "../lib/bindings";

export function makeSession(overrides: Partial<SessionSummary> = {}): SessionSummary {
  return {
    id: "s1",
    title: "Session 1",
    status: "running",
    worktreePath: null,
    issueUrl: null,
    branch: null,
    repoName: "alpha",
    ...overrides,
  };
}

export { createPaletteState } from "../state/palette";
export { createPickerState } from "../state/picker";
export { createContextMenuState } from "../state/context-menu";
export { createSetupStore } from "../state/setup";
export { createSessionsState } from "../state/sessions";
export { createSettingsStore } from "../state/settings";
export { createSidebarStore } from "../state/sidebar";
export { createReposStore } from "../state/repos";
export { createPtyStream } from "../state/pty-stream";
