import { useEffect, useRef } from "preact/hooks";
import { useFocusRestore, useFocusTrap } from "../../lib/use-focus-trap";

/// Default focus: whichever priority element mounts first wins. The repo
/// dropdown renders as soon as listRepos resolves; the search input
/// appears after listIssues. Without a prefill the dropdown beats the
/// search; with a prefill (drawer-launched) the search wins by default.
/// The modal itself is the fallback only — ref-attach handlers run during
/// commit and may have already claimed focus before the effect fires.
export function usePriorityFocus() {
  const modalRef = useRef<HTMLDivElement | null>(null);
  const focusedOnce = useRef(false);

  const claimFocus = (el: HTMLElement | null) => {
    if (el && !focusedOnce.current) {
      el.focus();
      focusedOnce.current = true;
    }
  };

  useEffect(() => {
    if (!focusedOnce.current) modalRef.current?.focus();
  }, []);
  useFocusRestore();
  useFocusTrap(modalRef);

  return {
    modalRef,
    selectRefAttach: (el: HTMLSelectElement | null) => claimFocus(el),
    searchRefAttach: (el: HTMLInputElement | null) => claimFocus(el),
  };
}
