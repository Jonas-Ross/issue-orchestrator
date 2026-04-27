import { useEffect, useRef } from "preact/hooks";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { commands } from "../lib/bindings";
import { attachTerminal, detachTerminal } from "../state/pty-stream";

interface Props {
  sessionId: string;
  active: boolean;
}

/// One xterm instance per session, mounted once and kept alive (hidden
/// via display:none when inactive) so scrollback survives tab switches.
export function TerminalView({ sessionId, active }: Props) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const refs = useRef<{ term: Terminal; fit: FitAddon } | null>(null);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;

    const term = new Terminal({
      fontFamily: "Menlo, Monaco, monospace",
      fontSize: 13,
      cursorBlink: true,
      theme: { background: "#0d0d0d", foreground: "#e6e6e6" },
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(host);
    fit.fit();

    term.onData((data) => {
      void commands.ptyWrite(sessionId, data);
    });

    attachTerminal(sessionId, term);
    refs.current = { term, fit };

    // Initial size sync to backend.
    void commands.ptyResize(sessionId, term.cols, term.rows);

    return () => {
      detachTerminal(sessionId);
      term.dispose();
      refs.current = null;
    };
  }, [sessionId]);

  // When this terminal becomes active, refit (the host has been hidden,
  // so its dimensions may have lagged) and re-sync the backend size.
  useEffect(() => {
    if (!active) return;
    const r = refs.current;
    if (!r) return;
    r.fit.fit();
    void commands.ptyResize(sessionId, r.term.cols, r.term.rows);
    r.term.focus();

    const onResize = () => {
      r.fit.fit();
      void commands.ptyResize(sessionId, r.term.cols, r.term.rows);
    };
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, [active, sessionId]);

  return (
    <div
      ref={hostRef}
      className="terminal-host"
      style={{ display: active ? "block" : "none" }}
    />
  );
}
