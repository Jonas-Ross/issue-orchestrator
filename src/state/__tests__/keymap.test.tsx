import { render } from "@testing-library/preact";
import { mockCommands } from "../../test/tauri-mock";
import { useKeymap } from "../keymap";
import { paletteOpen, closePalette } from "../palette";
import { pickerOpen, closePicker } from "../picker";
import { contextMenu, closeContextMenu, openContextMenu } from "../context-menu";
import { sidebarCollapsed } from "../sidebar";
import { settingsPanelOpen, closeSettings } from "../settings";
import { sessions, activeId, addSession, removeSession } from "../sessions";
import { makeSession } from "../../test/factories";

function TestHost() {
  useKeymap();
  return null;
}

function press(key: string, opts: KeyboardEventInit = {}) {
  window.dispatchEvent(
    new KeyboardEvent("keydown", {
      key,
      bubbles: true,
      cancelable: true,
      ...opts,
    }),
  );
}

beforeEach(() => {
  // singleton state must be reset; the keymap hook reads/writes module
  // singletons rather than scoped factory state.
  closePalette();
  closePicker();
  closeContextMenu();
  closeSettings();
  if (sidebarCollapsed.value) sidebarCollapsed.value = false;
  for (const s of [...sessions.value]) removeSession(s.id);
});

describe("useKeymap", () => {
  describe("⌘K — palette toggle", () => {
    it("opens the palette when closed", () => {
      mockCommands({});
      render(<TestHost />);
      press("k", { metaKey: true });
      expect(paletteOpen.value).toBe(true);
    });

    it("closes the palette when open", () => {
      mockCommands({});
      paletteOpen.value = true;
      render(<TestHost />);
      press("k", { metaKey: true });
      expect(paletteOpen.value).toBe(false);
    });
  });

  describe("Escape — closes top-most overlay", () => {
    it("closes context menu when present", () => {
      mockCommands({});
      openContextMenu({ x: 0, y: 0, items: [] });
      render(<TestHost />);
      press("Escape");
      expect(contextMenu.value).toBeNull();
    });

    it("closes settings panel when context menu is absent", () => {
      mockCommands({});
      settingsPanelOpen.value = true;
      render(<TestHost />);
      press("Escape");
      expect(settingsPanelOpen.value).toBe(false);
    });
  });

  describe("⌘N — opens picker", () => {
    it("opens picker with null repoName", () => {
      mockCommands({});
      render(<TestHost />);
      press("n", { metaKey: true });
      expect(pickerOpen.value).toEqual({ repoName: null });
    });
  });

  describe("⌘W — kills active session", () => {
    it("calls pty_kill with the active session id", () => {
      let killedId: string | undefined;
      mockCommands({
        pty_kill: (args: { id: string }) => {
          killedId = args.id;
          return null;
        },
      });
      addSession(makeSession({ id: "s-active" }));
      render(<TestHost />);
      press("w", { metaKey: true });
      expect(killedId).toBe("s-active");
    });

    it("is a no-op when no session is active", () => {
      let called = false;
      mockCommands({
        pty_kill: () => {
          called = true;
          return null;
        },
      });
      render(<TestHost />);
      press("w", { metaKey: true });
      expect(called).toBe(false);
    });
  });

  describe("⌘1..9 — switch active session by index", () => {
    it("activates the Nth session", () => {
      mockCommands({});
      addSession(makeSession({ id: "s1" }));
      addSession(makeSession({ id: "s2" }));
      addSession(makeSession({ id: "s3" }));
      render(<TestHost />);
      press("2", { metaKey: true });
      expect(activeId.value).toBe("s2");
    });

    it("ignores out-of-range numbers", () => {
      mockCommands({});
      addSession(makeSession({ id: "s1" }));
      render(<TestHost />);
      press("5", { metaKey: true });
      expect(activeId.value).toBe("s1");
    });
  });

  describe("⌘B — toggles sidebar", () => {
    it("flips the collapsed state", () => {
      mockCommands({});
      render(<TestHost />);
      press("b", { metaKey: true });
      expect(sidebarCollapsed.value).toBe(true);
      press("b", { metaKey: true });
      expect(sidebarCollapsed.value).toBe(false);
    });
  });

  describe("⌘, — toggles settings panel", () => {
    it("opens then closes the settings panel", () => {
      mockCommands({});
      render(<TestHost />);
      press(",", { metaKey: true });
      expect(settingsPanelOpen.value).toBe(true);
      press(",", { metaKey: true });
      expect(settingsPanelOpen.value).toBe(false);
    });
  });

  describe("⌘⇧B — debug bash spawn", () => {
    it("calls pty_spawn", () => {
      let spawned = false;
      mockCommands({
        pty_spawn: () => {
          spawned = true;
          // Tauri-specta wraps Result; the bindings layer handles shape.
          return {
            id: "dbg",
            title: "bash",
            status: "running",
            worktreePath: null,
            issueUrl: null,
            branch: null,
            repoName: null,
          };
        },
      });
      render(<TestHost />);
      press("b", { metaKey: true, shiftKey: true });
      expect(spawned).toBe(true);
    });
  });

  it("does not act on plain (non-meta) keystrokes", () => {
    mockCommands({});
    render(<TestHost />);
    press("k"); // no metaKey
    expect(paletteOpen.value).toBe(false);
  });
});
