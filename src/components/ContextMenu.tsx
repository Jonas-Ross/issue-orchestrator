import { useEffect } from "preact/hooks";
import { closeContextMenu, contextMenu } from "../state/context-menu";

export function ContextMenu() {
  const state = contextMenu.value;

  useEffect(() => {
    if (!state) return;
    const onDocClick = () => closeContextMenu();
    const onEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") closeContextMenu();
    };
    document.addEventListener("click", onDocClick);
    document.addEventListener("keydown", onEscape);
    return () => {
      document.removeEventListener("click", onDocClick);
      document.removeEventListener("keydown", onEscape);
    };
  }, [state]);

  if (!state) return null;

  return (
    <ul
      class="context-menu"
      style={{ left: `${state.x}px`, top: `${state.y}px` }}
      onClick={(e) => e.stopPropagation()}
    >
      {state.items.map((item, i) =>
        "separator" in item ? (
          <li key={`sep-${i}`} class="separator" />
        ) : (
          <li
            key={item.label}
            class={item.disabled ? "disabled" : undefined}
            onClick={() => {
              if (item.disabled) return;
              item.action();
              closeContextMenu();
            }}
          >
            {item.label}
          </li>
        ),
      )}
    </ul>
  );
}
