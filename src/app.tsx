import { useEffect } from "preact/hooks";
import { commands, events } from "./lib/bindings";
import { addSession, removeSession, setStatus } from "./state/sessions";
import { startPtyStream } from "./state/pty-stream";
import { setupState } from "./state/setup";
import { TabStrip } from "./components/TabStrip";
import { TerminalArea } from "./components/TerminalArea";
import { SetupPanel } from "./components/SetupPanel";
import { IssuePicker } from "./components/IssuePicker";
import { ContextMenu } from "./components/ContextMenu";
import { CommandPalette } from "./components/CommandPalette";
import { useKeymap } from "./state/keymap";
import { useNotifications } from "./state/notifications";

export function App() {
  useKeymap();
  useNotifications();

  useEffect(() => {
    startPtyStream();

    const unlistens: Array<() => void> = [];

    void (async () => {
      unlistens.push(
        await events.sessionAdded.listen((e) => addSession(e.payload)),
      );
      unlistens.push(
        await events.sessionRemoved.listen((e) =>
          removeSession(e.payload.sessionId),
        ),
      );
      unlistens.push(
        await events.statusChange.listen((e) =>
          setStatus(e.payload.sessionId, e.payload.status),
        ),
      );

      const sessionList = await commands.listSessions();
      if (sessionList.status === "ok") {
        for (const s of sessionList.data) addSession(s);
      }

      const setup = await commands.getSetupState();
      if (setup.status === "ok") {
        setupState.value = setup.data;
      }
    })();

    return () => {
      while (unlistens.length) unlistens.pop()?.();
    };
  }, []);

  return (
    <div class="app">
      <TabStrip />
      <TerminalArea />
      <IssuePicker />
      <CommandPalette />
      <SetupPanel />
      <ContextMenu />
    </div>
  );
}
