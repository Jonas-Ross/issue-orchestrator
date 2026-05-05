import { render, waitFor } from "@testing-library/preact";
import { useEffect } from "preact/hooks";
import { mockCommands } from "../../../test/tauri-mock";
import type { Issue } from "../types";
import { type PromptDraft, usePromptDraft } from "../use-prompt-draft";

const ISSUE_A: Issue = {
  number: 7,
  title: "Add tab strip",
  labels: ["feat"],
  url: "https://example.invalid/7",
};
const ISSUE_B: Issue = {
  number: 9,
  title: "Bug fix",
  labels: [],
  url: "https://example.invalid/9",
};

function configResult(template: string | null) {
  // Raw payload — `commands.getConfig` wraps this in {status, data} via
  // `typedError`, so the mock must NOT pre-wrap or it'll double-wrap.
  return {
    version: 1,
    worktreeRoot: "~/wt",
    repos: [],
    spawnPromptTemplate: template,
    setupDone: true,
  };
}

interface Captured {
  draft: PromptDraft | null;
}

/// Test host renders nothing but exposes the draft instance via a cap so
/// the test can interact with it. We use a wrapper object because plain
/// `let` is narrowed to its initial-null type by TS — the assignment
/// inside the callback is invisible to control-flow analysis.
function Host({ highlighted, cap }: { highlighted: Issue | null; cap: Captured }) {
  const draft = usePromptDraft(highlighted);
  useEffect(() => {
    cap.draft = draft;
  });
  return null;
}

describe("usePromptDraft", () => {
  it("renders the saved template with placeholders interpolated", async () => {
    mockCommands({
      get_config: () => configResult("Saved: {issue_title} (#{issue_number})"),
    });
    const cap: Captured = { draft: null };
    render(<Host highlighted={ISSUE_A} cap={cap} />);
    await waitFor(() => expect(cap.draft?.resolvedPrompt).toBe("Saved: Add tab strip (#7)"));
    expect(cap.draft?.isDirty).toBe(false);
    expect(cap.draft?.getOverrideFor(ISSUE_A)).toBeNull();
  });

  it("falls back to the built-in default when no template is saved", async () => {
    mockCommands({ get_config: () => configResult(null) });
    const cap: Captured = { draft: null };
    render(<Host highlighted={ISSUE_A} cap={cap} />);
    await waitFor(() =>
      expect(cap.draft?.resolvedPrompt).toBe(
        "Use the issue-team skill to implement issue #7 (Add tab strip).",
      ),
    );
  });

  it("setOverride only affects the issue it targets and survives navigation", async () => {
    mockCommands({ get_config: () => configResult(null) });
    const cap: Captured = { draft: null };
    const { rerender } = render(<Host highlighted={ISSUE_A} cap={cap} />);
    await waitFor(() => expect(cap.draft).not.toBeNull());

    cap.draft!.setOverride(ISSUE_A.number, "Custom for {issue_title}");
    await waitFor(() => expect(cap.draft?.resolvedPrompt).toBe("Custom for Add tab strip"));
    expect(cap.draft?.isDirty).toBe(true);
    expect(cap.draft?.getOverrideFor(ISSUE_A)).toBe("Custom for Add tab strip");

    // Switching issue: the override does NOT bleed across.
    rerender(<Host highlighted={ISSUE_B} cap={cap} />);
    await waitFor(() =>
      expect(cap.draft?.resolvedPrompt).toBe(
        "Use the issue-team skill to implement issue #9 (Bug fix).",
      ),
    );
    expect(cap.draft?.isDirty).toBe(false);
    expect(cap.draft?.getOverrideFor(ISSUE_B)).toBeNull();

    // Going back to A — the override is still there.
    rerender(<Host highlighted={ISSUE_A} cap={cap} />);
    await waitFor(() => expect(cap.draft?.resolvedPrompt).toBe("Custom for Add tab strip"));
  });

  it("reset drops the override for one issue", async () => {
    mockCommands({ get_config: () => configResult(null) });
    const cap: Captured = { draft: null };
    render(<Host highlighted={ISSUE_A} cap={cap} />);
    await waitFor(() => expect(cap.draft).not.toBeNull());
    cap.draft!.setOverride(ISSUE_A.number, "custom");
    await waitFor(() => expect(cap.draft?.isDirty).toBe(true));
    cap.draft!.reset(ISSUE_A.number);
    await waitFor(() => expect(cap.draft?.isDirty).toBe(false));
  });
});
