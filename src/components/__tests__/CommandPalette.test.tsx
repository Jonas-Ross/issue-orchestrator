import { fireEvent, render, screen } from "@testing-library/preact";
import { mockCommands } from "../../test/tauri-mock";
import { CommandPalette } from "../CommandPalette";
import { paletteOpen, closePalette } from "../../state/palette";
import { pickerOpen } from "../../state/picker";
import { sessions, activeId, addSession, removeSession } from "../../state/sessions";
import { repos } from "../../state/repos";
import { makeSession } from "../../test/factories";

beforeEach(() => {
  closePalette();
  pickerOpen.value = null;
  for (const s of [...sessions.value]) removeSession(s.id);
  activeId.value = null;
  repos.value = [];
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

  it("offers 'New Claude session (scratch)' and 'Debug bash' actions", () => {
    mockCommands({});
    paletteOpen.value = true;
    render(<CommandPalette />);
    expect(screen.getByText(/New Claude session \(scratch\)/i)).toBeInTheDocument();
    expect(screen.getByText(/Debug bash/i)).toBeInTheDocument();
  });

  it("clicking 'New Claude session (scratch)' calls claude_spawn with null repoName", () => {
    const calls: Array<{ repoName: string | null }> = [];
    mockCommands({
      claude_spawn: (args: { repoName: string | null }) => {
        calls.push({ repoName: args.repoName });
        return {
          id: "fake",
          title: "Claude",
          status: "running",
          worktreePath: null,
          issueUrl: null,
          branch: null,
          repoName: null,
        };
      },
    });
    paletteOpen.value = true;
    render(<CommandPalette />);
    fireEvent.click(screen.getByText(/New Claude session \(scratch\)/i));
    expect(calls).toEqual([{ repoName: null }]);
    expect(paletteOpen.value).toBe(false);
  });

  it("lists a 'New Claude session in <repo>' action per registered repo", () => {
    mockCommands({});
    repos.value = [
      { name: "alpha", path: "/p/alpha" },
      { name: "beta", path: "/p/beta" },
    ];
    paletteOpen.value = true;
    render(<CommandPalette />);
    expect(screen.getByText(/New Claude session in alpha/i)).toBeInTheDocument();
    expect(screen.getByText(/New Claude session in beta/i)).toBeInTheDocument();
  });

  it("clicking a per-repo Claude action passes the repo name to claude_spawn", () => {
    const calls: Array<{ repoName: string | null }> = [];
    mockCommands({
      claude_spawn: (args: { repoName: string | null }) => {
        calls.push({ repoName: args.repoName });
        return {
          id: "fake",
          title: `Claude · ${args.repoName}`,
          status: "running",
          worktreePath: null,
          issueUrl: null,
          branch: null,
          repoName: args.repoName,
        };
      },
    });
    repos.value = [{ name: "alpha", path: "/p/alpha" }];
    paletteOpen.value = true;
    render(<CommandPalette />);
    fireEvent.click(screen.getByText(/New Claude session in alpha/i));
    expect(calls).toEqual([{ repoName: "alpha" }]);
    expect(paletteOpen.value).toBe(false);
  });

  it("clicking 'Debug bash' calls pty_spawn", () => {
    let spawned = false;
    mockCommands({
      pty_spawn: () => {
        spawned = true;
        return {
          id: "bash",
          title: "bash",
          status: "running",
          worktreePath: null,
          issueUrl: null,
          branch: null,
          repoName: null,
        };
      },
    });
    paletteOpen.value = true;
    render(<CommandPalette />);
    fireEvent.click(screen.getByText(/Debug bash/i));
    expect(spawned).toBe(true);
    expect(paletteOpen.value).toBe(false);
  });
});
