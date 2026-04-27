import { useEffect } from "preact/hooks";
import { commands, events } from "./lib/bindings";
import { addSession, removeSession, setStatus } from "./state/sessions";
import { startPtyStream } from "./state/pty-stream";
import { TabStrip } from "./components/TabStrip";
import { TerminalArea } from "./components/TerminalArea";

export function App() {
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

      const result = await commands.listSessions();
      if (result.status === "ok") {
        for (const s of result.data) addSession(s);
      }
    })();

    return () => {
      while (unlistens.length) unlistens.pop()?.();
    };
  }, []);

  return (
    <div className="app">
      <TabStrip />
      <TerminalArea />
    </div>
  );
}
