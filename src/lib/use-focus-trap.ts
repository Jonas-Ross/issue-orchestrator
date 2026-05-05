import { useEffect } from "preact/hooks";
import type { RefObject } from "preact";

const FOCUSABLE_SELECTOR =
  'button:not([disabled]), input:not([disabled]):not([type="hidden"]), select:not([disabled]), textarea:not([disabled]), a[href], [tabindex]:not([tabindex="-1"])';

/// Tab trap for modal-style overlays. Cycles Tab / Shift+Tab through the
/// focusable elements inside `rootRef`, and pulls focus back inside if it
/// somehow escaped. Filters out elements that aren't currently visible
/// (offsetParent === null) so hidden affordances don't become tab dead
/// ends.
export function useFocusTrap(rootRef: RefObject<HTMLElement>) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Tab") return;
      const root = rootRef.current;
      if (!root) return;
      const focusables = Array.from(root.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
        (el) => el.offsetParent !== null || el === root,
      );
      if (focusables.length === 0) {
        e.preventDefault();
        root.focus();
        return;
      }
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      const active = document.activeElement as HTMLElement | null;
      const insideModal = active && root.contains(active);
      if (!insideModal) {
        e.preventDefault();
        (e.shiftKey ? last : first).focus();
        return;
      }
      if (e.shiftKey && active === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && active === last) {
        e.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [rootRef]);
}

/// Companion: when the modal mounts, save the previously focused element
/// so we can restore focus on unmount. Call this once at the top of the
/// modal's effect chain.
export function useFocusRestore() {
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    return () => {
      previous?.focus?.();
    };
  }, []);
}
