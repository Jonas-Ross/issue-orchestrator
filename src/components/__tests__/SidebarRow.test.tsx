import { fireEvent, render, screen } from "@testing-library/preact";
import { mockCommands } from "../../test/tauri-mock";
import { SidebarRow } from "../SidebarRow";
import { activeId, addSession, removeSession, sessions, setStatus } from "../../state/sessions";
import { makeSession } from "../../test/factories";

beforeEach(() => {
  for (const s of [...sessions.value]) removeSession(s.id);
  activeId.value = null;
});

describe("<SidebarRow /> — inline reply input", () => {
  // ── Criterion 1: input appears when needs_input + expanded ──────────────

  it("renders a text input when status is needs_input and collapsed is false", () => {
    mockCommands({});
    const session = makeSession({ id: "s1", status: "needs_input" });
    render(<SidebarRow session={session} collapsed={false} />);
    expect(screen.getByRole("textbox")).toBeInTheDocument();
  });

  // ── Criterion 2: input absent for non-needs_input statuses ──────────────

  it("does not render a text input when status is running", () => {
    mockCommands({});
    const session = makeSession({ id: "s1", status: "running" });
    render(<SidebarRow session={session} collapsed={false} />);
    expect(screen.queryByRole("textbox")).toBeNull();
  });

  it("does not render a text input when status is idle", () => {
    mockCommands({});
    const session = makeSession({ id: "s1", status: "idle" });
    render(<SidebarRow session={session} collapsed={false} />);
    expect(screen.queryByRole("textbox")).toBeNull();
  });

  it("does not render a text input when status is spawning", () => {
    mockCommands({});
    const session = makeSession({ id: "s1", status: "spawning" });
    render(<SidebarRow session={session} collapsed={false} />);
    expect(screen.queryByRole("textbox")).toBeNull();
  });

  it("does not render a text input when status is exited", () => {
    mockCommands({});
    const session = makeSession({ id: "s1", status: "exited" });
    render(<SidebarRow session={session} collapsed={false} />);
    expect(screen.queryByRole("textbox")).toBeNull();
  });

  // ── Criterion 3: input absent on collapsed layout regardless of status ──

  it("does not render a text input when collapsed is true even if status is needs_input", () => {
    mockCommands({});
    const session = makeSession({ id: "s1", status: "needs_input" });
    render(<SidebarRow session={session} collapsed={true} />);
    expect(screen.queryByRole("textbox")).toBeNull();
  });

  // ── Criterion 4: Enter calls pty_write with id and text + newline ────────

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

  // ── Criterion 5: input value reset to empty after Enter ──────────────────

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

  // ── Criterion 6: Esc clears value without calling pty_write ──────────────

  it("pressing Esc clears the input value without calling pty_write", () => {
    // mockCommands with no pty_write handler — any pty_write invocation throws
    mockCommands({});
    const session = makeSession({ id: "s1", status: "needs_input" });
    render(<SidebarRow session={session} collapsed={false} />);
    const input = screen.getByRole("textbox") as HTMLInputElement;
    fireEvent.input(input, { target: { value: "draft text" } });
    // Must not throw (pty_write unmocked means it would throw if called)
    fireEvent.keyDown(input, { key: "Escape" });
    expect(input.value).toBe("");
  });

  // ── Criterion 7: status leaving needs_input removes input; re-entry gives fresh empty input ──

  it("removes the input from DOM when status transitions away from needs_input", () => {
    mockCommands({});
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

    // Type a draft but do NOT press Enter
    const input = screen.getByRole("textbox") as HTMLInputElement;
    fireEvent.input(input, { target: { value: "stale draft" } });

    // Status leaves needs_input — input removed
    setStatus("s1", "running");
    rerender(<SidebarRow session={sessions.value[0]!} collapsed={false} />);
    expect(screen.queryByRole("textbox")).toBeNull();

    // Status returns to needs_input — new input must be empty
    setStatus("s1", "needs_input");
    rerender(<SidebarRow session={sessions.value[0]!} collapsed={false} />);
    const freshInput = screen.getByRole("textbox") as HTMLInputElement;
    expect(freshInput.value).toBe("");
  });

  // ── Criterion 8: input does NOT auto-focus on render ──────────────────────

  it("does not steal focus when the reply input first appears", () => {
    mockCommands({});
    const session = makeSession({ id: "s1", status: "needs_input" });
    render(<SidebarRow session={session} collapsed={false} />);
    const input = screen.getByRole("textbox");
    expect(document.activeElement).not.toBe(input);
  });

  it("does not steal focus when status transitions into needs_input", () => {
    mockCommands({});
    const session = makeSession({ id: "s1", status: "running" });
    addSession(session);
    const { rerender } = render(<SidebarRow session={sessions.value[0]!} collapsed={false} />);

    setStatus("s1", "needs_input");
    rerender(<SidebarRow session={sessions.value[0]!} collapsed={false} />);
    const input = screen.getByRole("textbox");
    expect(document.activeElement).not.toBe(input);
  });

  // ── Criterion 9: click inside input does not change activeId ─────────────

  it("clicking inside the input does not bubble to the row and does not change activeId", () => {
    mockCommands({});
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

  // ── Criterion 10: keypresses inside input do not affect activeId ──────────

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
    // activeId must remain s1 — keydown on input must not have selected s2
    expect(activeId.value).toBe("s1");
  });

  // ── Criterion 11: regression — existing close button still calls pty_kill ──

  it("clicking the close button calls pty_kill with the session id", () => {
    let killedId: string | undefined;
    mockCommands({
      pty_kill: (args: { id: string }) => {
        killedId = args.id;
        return null;
      },
    });
    const session = makeSession({ id: "s1", status: "needs_input" });
    render(<SidebarRow session={session} collapsed={false} />);
    const closeBtn = document.querySelector(".sb-row-close") as HTMLElement;
    expect(closeBtn).not.toBeNull();
    fireEvent.click(closeBtn);
    expect(killedId).toBe("s1");
  });

  it("clicking the close button when status is running still calls pty_kill", () => {
    let killedId: string | undefined;
    mockCommands({
      pty_kill: (args: { id: string }) => {
        killedId = args.id;
        return null;
      },
    });
    const session = makeSession({ id: "s2", status: "running" });
    render(<SidebarRow session={session} collapsed={false} />);
    const closeBtn = document.querySelector(".sb-row-close") as HTMLElement;
    expect(closeBtn).not.toBeNull();
    fireEvent.click(closeBtn);
    expect(killedId).toBe("s2");
  });

  // ── Criterion 12: reply input carries the documented CSS class ──────────

  it("renders the reply input with the sb-row-reply CSS class", () => {
    mockCommands({});
    const session = makeSession({ id: "s1", status: "needs_input" });
    render(<SidebarRow session={session} collapsed={false} />);
    const input = document.querySelector(".sb-row-reply");
    expect(input).not.toBeNull();
  });

  // ── Criterion 13: needs class still applied on the row ──────────────────

  it("applies the needs CSS class to the row when status is needs_input", () => {
    mockCommands({});
    const session = makeSession({ id: "s1", status: "needs_input" });
    const { container } = render(<SidebarRow session={session} collapsed={false} />);
    const row = container.firstChild as HTMLElement;
    expect(row.classList.contains("needs")).toBe(true);
  });

  it("does not apply the needs CSS class when status is running", () => {
    mockCommands({});
    const session = makeSession({ id: "s1", status: "running" });
    const { container } = render(<SidebarRow session={session} collapsed={false} />);
    const row = container.firstChild as HTMLElement;
    expect(row.classList.contains("needs")).toBe(false);
  });
});
