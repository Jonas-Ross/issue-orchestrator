import { fireEvent, render, screen } from "@testing-library/preact";
import { mockCommands } from "../../test/tauri-mock";
import { SidebarRow } from "../SidebarRow";
import {
  activeId,
  addSession,
  removeSession,
  sessions,
  setStatus,
  setPrStatus,
} from "../../state/sessions";
import { makeSession } from "../../test/factories";

beforeEach(() => {
  for (const s of [...sessions.value]) removeSession(s.id);
  activeId.value = null;
  mockCommands({});
});

describe("<SidebarRow /> — inline reply input", () => {
  it("renders a text input when status is needs_input and collapsed is false", () => {
    const session = makeSession({ id: "s1", status: "needs_input" });
    render(<SidebarRow session={session} collapsed={false} />);
    expect(screen.getByRole("textbox")).toBeInTheDocument();
  });

  it.each(["running", "idle", "spawning", "exited"] as const)(
    "does not render a text input when status is %s",
    (status) => {
      const session = makeSession({ id: "s1", status });
      render(<SidebarRow session={session} collapsed={false} />);
      expect(screen.queryByRole("textbox")).toBeNull();
    },
  );

  it("does not render a text input when collapsed is true even if status is needs_input", () => {
    const session = makeSession({ id: "s1", status: "needs_input" });
    render(<SidebarRow session={session} collapsed={true} />);
    expect(screen.queryByRole("textbox")).toBeNull();
  });

  it("pressing Enter calls pty_write with session id and typed text followed by newline", () => {
    let recorded: { id: string; data: string } | undefined;
    mockCommands({
      pty_write: (args: { id: string; data: string }) => {
        recorded = args;
        return null;
      },
    });
    const session = makeSession({ id: "s1", status: "needs_input" });
    render(<SidebarRow session={session} collapsed={false} />);
    const input = screen.getByRole("textbox") as HTMLInputElement;
    fireEvent.input(input, { target: { value: "yes" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(recorded).toBeDefined();
    expect(recorded!.id).toBe("s1");
    expect(recorded!.data).toBe("yes\n");
  });

  it("does not dispatch pty_write when Enter fires during IME composition", () => {
    // pty_write left unmocked — mockCommands would throw if Enter dispatched it mid-composition.
    const session = makeSession({ id: "s1", status: "needs_input" });
    render(<SidebarRow session={session} collapsed={false} />);
    const input = screen.getByRole("textbox") as HTMLInputElement;
    fireEvent.input(input, { target: { value: "ni" } });
    fireEvent.keyDown(input, { key: "Enter", isComposing: true });
    expect(input.value).toBe("ni");
  });

  it("clears the input value after Enter is pressed", () => {
    mockCommands({
      pty_write: () => null,
    });
    const session = makeSession({ id: "s1", status: "needs_input" });
    render(<SidebarRow session={session} collapsed={false} />);
    const input = screen.getByRole("textbox") as HTMLInputElement;
    fireEvent.input(input, { target: { value: "hello" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(input.value).toBe("");
  });

  it("pressing Esc clears the input value without calling pty_write", () => {
    const session = makeSession({ id: "s1", status: "needs_input" });
    render(<SidebarRow session={session} collapsed={false} />);
    const input = screen.getByRole("textbox") as HTMLInputElement;
    fireEvent.input(input, { target: { value: "draft text" } });
    // pty_write is unmocked — mockCommands would throw if Esc dispatched it.
    fireEvent.keyDown(input, { key: "Escape" });
    expect(input.value).toBe("");
  });

  it("removes the input from DOM when status transitions away from needs_input", () => {
    const session = makeSession({ id: "s1", status: "needs_input" });
    addSession(session);
    const { rerender } = render(<SidebarRow session={sessions.value[0]!} collapsed={false} />);
    expect(screen.getByRole("textbox")).toBeInTheDocument();

    setStatus("s1", "running");
    rerender(<SidebarRow session={sessions.value[0]!} collapsed={false} />);
    expect(screen.queryByRole("textbox")).toBeNull();
  });

  it("renders an empty input when re-entering needs_input after a draft was typed", () => {
    mockCommands({
      pty_write: () => null,
    });
    const session = makeSession({ id: "s1", status: "needs_input" });
    addSession(session);
    const { rerender } = render(<SidebarRow session={sessions.value[0]!} collapsed={false} />);

    const input = screen.getByRole("textbox") as HTMLInputElement;
    fireEvent.input(input, { target: { value: "stale draft" } });

    setStatus("s1", "running");
    rerender(<SidebarRow session={sessions.value[0]!} collapsed={false} />);
    expect(screen.queryByRole("textbox")).toBeNull();

    setStatus("s1", "needs_input");
    rerender(<SidebarRow session={sessions.value[0]!} collapsed={false} />);
    const freshInput = screen.getByRole("textbox") as HTMLInputElement;
    expect(freshInput.value).toBe("");
  });

  it("does not steal focus when the reply input first appears", () => {
    const session = makeSession({ id: "s1", status: "needs_input" });
    render(<SidebarRow session={session} collapsed={false} />);
    const input = screen.getByRole("textbox");
    expect(document.activeElement).not.toBe(input);
  });

  it("does not steal focus when status transitions into needs_input", () => {
    const session = makeSession({ id: "s1", status: "running" });
    addSession(session);
    const { rerender } = render(<SidebarRow session={sessions.value[0]!} collapsed={false} />);

    setStatus("s1", "needs_input");
    rerender(<SidebarRow session={sessions.value[0]!} collapsed={false} />);
    const input = screen.getByRole("textbox");
    expect(document.activeElement).not.toBe(input);
  });

  it("clicking inside the input does not bubble to the row and does not change activeId", () => {
    const s1 = makeSession({ id: "s1", status: "running" });
    const s2 = makeSession({ id: "s2", status: "needs_input" });
    addSession(s1);
    addSession(s2);
    activeId.value = "s1";

    render(<SidebarRow session={s2} collapsed={false} />);
    const input = screen.getByRole("textbox");
    fireEvent.click(input);
    expect(activeId.value).toBe("s1");
  });

  it("pressing keys inside the input does not bubble to the row handler", () => {
    mockCommands({
      pty_write: () => null,
    });
    const s1 = makeSession({ id: "s1", status: "running" });
    const s2 = makeSession({ id: "s2", status: "needs_input" });
    addSession(s1);
    addSession(s2);
    activeId.value = "s1";

    render(<SidebarRow session={s2} collapsed={false} />);
    const input = screen.getByRole("textbox") as HTMLInputElement;
    fireEvent.input(input, { target: { value: "hello" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(activeId.value).toBe("s1");
  });

  it.each([
    ["needs_input", "s1"],
    ["running", "s2"],
  ] as const)("clicking the close button calls pty_kill (status=%s)", (status, id) => {
    let killedId: string | undefined;
    mockCommands({
      pty_kill: (args: { id: string }) => {
        killedId = args.id;
        return null;
      },
    });
    const session = makeSession({ id, status });
    render(<SidebarRow session={session} collapsed={false} />);
    const closeBtn = document.querySelector(".sb-row-close") as HTMLElement;
    expect(closeBtn).not.toBeNull();
    fireEvent.click(closeBtn);
    expect(killedId).toBe(id);
  });

  it("renders the reply input with the sb-row-reply CSS class", () => {
    const session = makeSession({ id: "s1", status: "needs_input" });
    render(<SidebarRow session={session} collapsed={false} />);
    const input = document.querySelector(".sb-row-reply");
    expect(input).not.toBeNull();
  });

  it("applies the needs CSS class to the row when status is needs_input", () => {
    const session = makeSession({ id: "s1", status: "needs_input" });
    const { container } = render(<SidebarRow session={session} collapsed={false} />);
    const row = container.firstChild as HTMLElement;
    expect(row.classList.contains("needs")).toBe(true);
  });

  it("does not apply the needs CSS class when status is running", () => {
    const session = makeSession({ id: "s1", status: "running" });
    const { container } = render(<SidebarRow session={session} collapsed={false} />);
    const row = container.firstChild as HTMLElement;
    expect(row.classList.contains("needs")).toBe(false);
  });
});

describe("<SidebarRow /> — PR chip", () => {
  beforeEach(() => {
    for (const s of [...sessions.value]) removeSession(s.id);
    activeId.value = null;
    // Allow open() calls (plugin:shell|open) to pass through silently so chip
    // click tests don't throw on unmocked commands.
    mockCommands({ "plugin:shell|open": () => null }); // allow PrChip click-through
  });

  it("renders the PR chip when prStatus is present and session has a branch", () => {
    const session = makeSession({
      id: "s1",
      branch: "feature/my-branch",
      prStatus: { number: 42, url: "https://github.com/foo/bar/pull/42", checks: "pass" },
    });
    render(<SidebarRow session={session} collapsed={false} />);
    expect(document.querySelector(".pr-chip")).not.toBeNull();
    expect(screen.getByText(/PR #42/)).toBeInTheDocument();
  });

  it("does not render the PR chip when prStatus is null", () => {
    const session = makeSession({
      id: "s1",
      branch: "feature/my-branch",
      prStatus: null,
    });
    render(<SidebarRow session={session} collapsed={false} />);
    expect(document.querySelector(".pr-chip")).toBeNull();
  });

  it("applies pr-chip-fail class when checks is fail", () => {
    const session = makeSession({
      id: "s1",
      branch: "feature/failing",
      prStatus: { number: 7, url: "https://github.com/foo/bar/pull/7", checks: "fail" },
    });
    render(<SidebarRow session={session} collapsed={false} />);
    const chip = document.querySelector(".pr-chip");
    expect(chip).not.toBeNull();
    expect(chip!.classList.contains("pr-chip-fail")).toBe(true);
  });

  it("applies pr-chip-pending class when checks is pending", () => {
    const session = makeSession({
      id: "s1",
      branch: "feature/running",
      prStatus: { number: 3, url: "https://github.com/foo/bar/pull/3", checks: "pending" },
    });
    render(<SidebarRow session={session} collapsed={false} />);
    const chip = document.querySelector(".pr-chip");
    expect(chip!.classList.contains("pr-chip-pending")).toBe(true);
  });

  it("clicking the PR chip does not change the active session", () => {
    const s1 = makeSession({ id: "s1", status: "running" });
    const s2 = makeSession({
      id: "s2",
      status: "running",
      branch: "feature/has-pr",
      prStatus: { number: 10, url: "https://github.com/foo/bar/pull/10", checks: "pass" },
    });
    addSession(s1);
    addSession(s2);
    activeId.value = "s1";

    render(<SidebarRow session={s2} collapsed={false} />);
    const chip = document.querySelector(".pr-chip") as HTMLElement;
    fireEvent.click(chip);
    expect(activeId.value).toBe("s1");
  });

  it("does not render the PR chip for shell sessions (no branch)", () => {
    const session = makeSession({
      id: "s1",
      branch: null,
      issueUrl: null,
      prStatus: { number: 99, url: "https://github.com/foo/bar/pull/99", checks: "pass" },
    });
    render(<SidebarRow session={session} collapsed={false} />);
    // Shell sessions don't render the line3 block at all, so the chip can't appear.
    expect(document.querySelector(".pr-chip")).toBeNull();
  });

  // AC #5: when prStatus transitions Some → null the chip must disappear
  it("removes the PR chip when prStatus transitions from Some to null", () => {
    const session = makeSession({
      id: "ac5",
      branch: "feature/was-open",
      prStatus: { number: 55, url: "https://github.com/foo/bar/pull/55", checks: "pass" },
    });
    addSession(session);
    const { rerender } = render(
      <SidebarRow session={sessions.value.find((s) => s.id === "ac5")!} collapsed={false} />,
    );
    expect(document.querySelector(".pr-chip")).not.toBeNull();

    // Simulate enrichment actor clearing prStatus (e.g. PR was merged/closed)
    setPrStatus("ac5", null);
    rerender(
      <SidebarRow session={sessions.value.find((s) => s.id === "ac5")!} collapsed={false} />,
    );
    expect(document.querySelector(".pr-chip")).toBeNull();
  });
});
