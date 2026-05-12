import { fireEvent, render, screen, waitFor } from "@testing-library/preact";
import { mockCommands } from "../../test/tauri-mock";
import { DiffView } from "../DiffView";
import { clearForSession } from "../../state/view-mode";

beforeEach(() => {
  for (const id of ["s-empty", "s-diff", "s-err"]) clearForSession(id);
});

describe("<DiffView />", () => {
  it("renders 'Working tree clean' when git_diff returns empty", async () => {
    mockCommands({ git_diff: () => "" });
    render(<DiffView sessionId="s-empty" worktreePath="/x" />);
    expect(await screen.findByText("Working tree clean")).toBeInTheDocument();
  });

  it("renders classified line spans for a unified diff", async () => {
    const diff = [
      "diff --git a/x.txt b/x.txt",
      "index 1111..2222 100644",
      "--- a/x.txt",
      "+++ b/x.txt",
      "@@ -1,2 +1,2 @@",
      " context",
      "-old",
      "+new",
    ].join("\n");
    mockCommands({ git_diff: () => diff });
    const { container } = render(<DiffView sessionId="s-diff" worktreePath="/x" />);

    await waitFor(() => {
      expect(container.querySelector(".diff-line-add")).not.toBeNull();
    });
    expect(container.querySelector(".diff-line-del")?.textContent).toContain("-old");
    expect(container.querySelector(".diff-line-add")?.textContent).toContain("+new");
    expect(container.querySelector(".diff-line-hunk")?.textContent).toContain("@@");
    expect(container.querySelectorAll(".diff-line-meta").length).toBeGreaterThanOrEqual(2);
    expect(container.querySelector(".diff-line-context")?.textContent).toContain("context");
  });

  it("shows error state with retry button on failure", async () => {
    let calls = 0;
    mockCommands({
      git_diff: () => {
        calls++;
        if (calls === 1) throw "fatal: not a git repository";
        return "";
      },
    });
    render(<DiffView sessionId="s-err" worktreePath="/x" />);
    const retry = await screen.findByRole("button", { name: /retry/i });
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByText(/fatal: not a git repository/)).toBeInTheDocument();
    fireEvent.click(retry);
    await waitFor(() => expect(calls).toBe(2));
  });
});
