import { useEffect } from "preact/hooks";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { commands, events } from "./lib/bindings";
import { addSession, removeSession, setStatus, updateSession } from "./state/sessions";
import { startPtyStream } from "./state/pty-stream";
import { setupState } from "./state/setup";
import { loadRepos } from "./state/repos";
import { Sidebar } from "./components/Sidebar";
import { StatusBar } from "./components/StatusBar";
import { TerminalArea } from "./components/TerminalArea";
import { SetupPanel } from "./components/SetupPanel";
import { IssuePicker } from "./components/IssuePicker";
import { ContextMenu } from "./components/ContextMenu";
import { CommandPalette } from "./components/CommandPalette";
import { SettingsPanel } from "./components/SettingsPanel";
import { useKeymap } from "./state/keymap";
import { useNotifications } from "./state/notifications";

export function App() {
  useKeymap();
  useNotifications();

  useEffect(() => {
    startPtyStream();

    const unlistens: Array<() => void> = [];

    void (async () => {
      unlistens.push(await events.sessionAdded.listen((e) => addSession(e.payload)));
      unlistens.push(await events.sessionRemoved.listen((e) => removeSession(e.payload.sessionId)));
      unlistens.push(await events.sessionUpdated.listen((e) => updateSession(e.payload)));
      unlistens.push(
        await events.statusChange.listen((e) => setStatus(e.payload.sessionId, e.payload.status)),
      );

      const sessionList = await commands.listSessions();
      if (sessionList.status === "ok") {
        for (const s of sessionList.data) addSession(s);
      }

      const setup = await commands.getSetupState();
      if (setup.status === "ok") {
        setupState.value = setup.data;
      }

      await loadRepos();
    })();

    return () => {
      while (unlistens.length) unlistens.pop()?.();
    };
  }, []);

  // startDragging() requires core:window:allow-start-dragging in
  // capabilities/default.json — easy to break in another file.
  const onTitlebarMouseDown = (e: MouseEvent) => {
    if (e.buttons !== 1) return;
    const win = getCurrentWindow();
    const op = e.detail === 2 ? win.toggleMaximize() : win.startDragging();
    op.catch((err) => console.error("titlebar drag failed:", err));
  };

  return (
    <div class="app">
      <div class="titlebar" onMouseDown={onTitlebarMouseDown} />
      <div class="app-body">
        <Sidebar />
        <TerminalArea />
      </div>
      <StatusBar />
      <IssuePicker />
      <CommandPalette />
      <SettingsPanel />
      <SetupPanel />
      <ContextMenu />
    </div>
  );
}
