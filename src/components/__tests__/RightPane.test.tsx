import { fireEvent, render, screen } from "@testing-library/preact";
import { mockCommands } from "../../test/tauri-mock";

// xterm + the DiffView fetch path aren't what this test is about — stub them.
vi.mock("../TerminalView", () => ({
  TerminalView: ({ sessionId }: { sessionId: string }) => (
    <div data-testid={`terminal-${sessionId}`} />
  ),
}));
vi.mock("../DiffView", () => ({
  DiffView: ({ sessionId }: { sessionId: string }) => <div data-testid={`diff-${sessionId}`} />,
}));

import { RightPane } from "../RightPane";
import { clearForSession, getMode } from "../../state/view-mode";

beforeEach(() => {
  mockCommands({});
  for (const id of ["s-wt", "s-nowt"]) clearForSession(id);
});

describe("<RightPane />", () => {
  it("renders Terminal and Diff tabs when worktreePath is provided", () => {
    render(<RightPane sessionId="s-wt" active={true} worktreePath="/repo" />);
    expect(screen.getByRole("tab", { name: "Terminal" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Diff" })).toBeInTheDocument();
  });

  it("hides the Diff tab when worktreePath is null", () => {
    render(<RightPane sessionId="s-nowt" active={true} worktreePath={null} />);
    expect(screen.getByRole("tab", { name: "Terminal" })).toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "Diff" })).toBeNull();
  });

  it("clicking the Diff tab switches mode and mounts DiffView", () => {
    render(<RightPane sessionId="s-wt" active={true} worktreePath="/repo" />);
    expect(screen.queryByTestId("diff-s-wt")).toBeNull();
    fireEvent.click(screen.getByRole("tab", { name: "Diff" }));
    expect(getMode("s-wt")).toBe("diff");
    expect(screen.getByTestId("diff-s-wt")).toBeInTheDocument();
  });

  it("Diff tab exposes a refresh button only while diff mode is active", () => {
    render(<RightPane sessionId="s-wt" active={true} worktreePath="/repo" />);
    expect(screen.queryByLabelText("Refresh diff")).toBeNull();
    fireEvent.click(screen.getByRole("tab", { name: "Diff" }));
    expect(screen.getByLabelText("Refresh diff")).toBeInTheDocument();
  });
});
