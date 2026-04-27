import { useEffect } from "preact/hooks";
import { commands } from "../lib/bindings";
import { activeId, sessions } from "./sessions";
import { closePicker, openPicker, pickerOpen } from "./picker";
import { closePalette, paletteOpen, togglePalette } from "./palette";
import { closeContextMenu, contextMenu } from "./context-menu";

/// App-wide keyboard shortcuts. Installed once on the window in capture
/// phase so Cmd-modified events are handled before xterm sees them.
/// Plain typing in the terminal is unaffected because every binding
/// requires `metaKey` (or is `Escape`, which xterm doesn't normally
/// translate to terminal input).
export function useKeymap() {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (contextMenu.value) {
          closeContextMenu();
          e.preventDefault();
          e.stopPropagation();
          return;
        }
        if (paletteOpen.value) {
          closePalette();
          e.preventDefault();
          e.stopPropagation();
          return;
        }
        if (pickerOpen.value) {
          closePicker();
          e.preventDefault();
          e.stopPropagation();
          return;
        }
        return;
      }

      if (!e.metaKey) return;

      const list = sessions.value;
      const handle = (action: () => void) => {
        e.preventDefault();
        e.stopPropagation();
        action();
      };

      switch (e.key) {
        case "t":
          return handle(() => {
            closePalette();
            openPicker();
          });
        case "w":
          return handle(() => {
            const id = activeId.value;
            if (id) void commands.ptyKill(id);
          });
        case "k":
          return handle(() => togglePalette());
        case "]":
          return handle(() => {
            if (list.length === 0) return;
            const idx = list.findIndex((s) => s.id === activeId.value);
            const next = list[(idx + 1) % list.length];
            activeId.value = next.id;
          });
        case "[":
          return handle(() => {
            if (list.length === 0) return;
            const idx = list.findIndex((s) => s.id === activeId.value);
            const next = list[(idx - 1 + list.length) % list.length];
            activeId.value = next.id;
          });
        default: {
          if (/^[1-9]$/.test(e.key)) {
            const n = parseInt(e.key, 10);
            return handle(() => {
              const target = list[n - 1];
              if (target) activeId.value = target.id;
            });
          }
        }
      }
    };

    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, []);
}
