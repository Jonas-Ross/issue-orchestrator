interface Props {
  canDecide: boolean;
  recommending: boolean;
  onDecide: () => void;
  onClose: () => void;
}

export function IssuePickerHeader({ canDecide, recommending, onDecide, onClose }: Props) {
  return (
    <div class="modal-header">
      <h2>Pick an issue</h2>
      <div class="modal-header-actions">
        <button
          type="button"
          class="decide-btn"
          disabled={!canDecide}
          title="Ask Claude to recommend the best next issue"
          onClick={onDecide}
        >
          <span style={{ fontFamily: "var(--font-mono)", fontSize: 10 }}>✦</span>
          {recommending ? "Thinking…" : "Suggest a task"}
        </button>
        <button type="button" class="close" onClick={onClose}>
          ×
        </button>
      </div>
    </div>
  );
}
