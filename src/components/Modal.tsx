import type { CSSProperties, ComponentChildren, Ref } from "preact";

interface Props {
  onClose: () => void;
  children: ComponentChildren;
  /// Inner dialog className. Defaults to "modal"; override with
  /// "settings-shell" or similar when the dialog has its own layout.
  dialogClass?: string;
  dialogRef?: Ref<HTMLDivElement>;
  style?: CSSProperties;
  /// `tabIndex={-1}` makes the dialog focusable as a fallback when no
  /// child claims focus. Pass through unchanged from the consumer.
  tabIndex?: number;
}

/// Click-outside-to-close modal wrapper. Shared by every dialog in the
/// app: ⌘K palette, settings panel, issue picker. Inner `stopPropagation`
/// ensures clicks inside the dialog don't bubble to the overlay's close
/// handler.
export function Modal({
  onClose,
  children,
  dialogClass = "modal",
  dialogRef,
  style,
  tabIndex,
}: Props) {
  return (
    <div class="modal-overlay" onClick={onClose}>
      <div
        class={dialogClass}
        ref={dialogRef}
        style={style}
        tabIndex={tabIndex}
        onClick={(e) => e.stopPropagation()}
      >
        {children}
      </div>
    </div>
  );
}
