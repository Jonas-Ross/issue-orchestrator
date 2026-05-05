import { fireEvent, render, screen } from "@testing-library/preact";
import { mockCommands } from "../../test/tauri-mock";
import { CommandPalette } from "../CommandPalette";
import { paletteOpen, closePalette } from "../../state/palette";
import { pickerOpen } from "../../state/picker";
import { sessions, activeId, addSession, removeSession } from "../../state/sessions";
import { makeSession } from "../../test/factories";

beforeEach(() => {
  closePalette();
  pickerOpen.value = null;
  for (const s of [...sessions.value]) removeSession(s.id);
  activeId.value = null;
});

describe("<CommandPalette />", () => {
  it("renders nothing when paletteOpen is false", () => {
    mockCommands({});
    const { container } = render(<CommandPalette />);
    expect(container.firstChild).toBeNull();
  });

  it("renders the search input when open", () => {
    mockCommands({});
    paletteOpen.value = true;
    render(<CommandPalette />);
    expect(screen.getByPlaceholderText(/Switch session, run command/i)).toBeInTheDocument();
  });

  it("always offers the 'New issue session' action", () => {
    mockCommands({});
    paletteOpen.value = true;
    render(<CommandPalette />);
    expect(screen.getByText(/New issue session/i)).toBeInTheDocument();
  });

  it("clicking 'New issue session' closes the palette and opens the picker", () => {
    mockCommands({});
    paletteOpen.value = true;
    render(<CommandPalette />);
    fireEvent.click(screen.getByText(/New issue session/i));
    expect(paletteOpen.value).toBe(false);
    expect(pickerOpen.value).toEqual({ repoName: null });
  });

  it("lists each session as a 'Switch to' action", () => {
    mockCommands({});
    addSession(makeSession({ id: "s1", title: "Build the rocket" }));
    addSession(makeSession({ id: "s2", title: "Land the rocket" }));
    paletteOpen.value = true;
    render(<CommandPalette />);
    expect(screen.getByText(/Switch to: Build the rocket/)).toBeInTheDocument();
    expect(screen.getByText(/Switch to: Land the rocket/)).toBeInTheDocument();
  });

  it("clicking a 'Switch to' action sets activeId and closes the palette", () => {
    mockCommands({});
    addSession(makeSession({ id: "s1", title: "alpha" }));
    addSession(makeSession({ id: "s2", title: "beta" }));
    paletteOpen.value = true;
    render(<CommandPalette />);
    fireEvent.click(screen.getByText(/Switch to: beta/));
    expect(activeId.value).toBe("s2");
    expect(paletteOpen.value).toBe(false);
  });

  it("query filtering narrows the visible actions", () => {
    mockCommands({});
    addSession(makeSession({ id: "s1", title: "alpha" }));
    addSession(makeSession({ id: "s2", title: "beta" }));
    paletteOpen.value = true;
    render(<CommandPalette />);
    const input = screen.getByPlaceholderText(/Switch session/i) as HTMLInputElement;
    fireEvent.input(input, { target: { value: "beta" } });
    expect(screen.queryByText(/Switch to: alpha/)).toBeNull();
    expect(screen.getByText(/Switch to: beta/)).toBeInTheDocument();
  });

  it("Enter on the input runs the active action", () => {
    mockCommands({});
    paletteOpen.value = true;
    render(<CommandPalette />);
    const input = screen.getByPlaceholderText(/Switch session/i);
    // first action by default is "New issue session"
    fireEvent.keyDown(input, { key: "Enter" });
    expect(pickerOpen.value).toEqual({ repoName: null });
    expect(paletteOpen.value).toBe(false);
  });

  it("offers 'Kill active session' only when a session is active", () => {
    mockCommands({});
    paletteOpen.value = true;
    const { rerender } = render(<CommandPalette />);
    expect(screen.queryByText(/Kill active session/)).toBeNull();

    addSession(makeSession({ id: "s1" }));
    rerender(<CommandPalette />);
    expect(screen.getByText(/Kill active session/)).toBeInTheDocument();
  });

  it("clicking 'Kill active session' calls pty_kill with the active id", () => {
    let killedId: string | undefined;
    mockCommands({
      pty_kill: (args: { id: string }) => {
        killedId = args.id;
        return null;
      },
    });
    addSession(makeSession({ id: "s1" }));
    paletteOpen.value = true;
    render(<CommandPalette />);
    fireEvent.click(screen.getByText(/Kill active session/));
    expect(killedId).toBe("s1");
    expect(paletteOpen.value).toBe(false);
  });
});
