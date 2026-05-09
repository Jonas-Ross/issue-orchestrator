import { useEffect, useRef } from "preact/hooks";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { commands } from "../lib/bindings";
import { attachTerminal, detachTerminal } from "../state/pty-stream";

interface Props {
  sessionId: string;
  active: boolean;
}

function syncResize(sessionId: string, term: Terminal, fit: FitAddon) {
  fit.fit();
  void commands.ptyResize(sessionId, term.cols, term.rows);
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
    // Initial fit — font may not be loaded yet; re-fit after fonts.ready.
    fit.fit();

    term.onData((data) => {
      void commands.ptyWrite(sessionId, data);
    });

    attachTerminal(sessionId, term);
    refs.current = { term, fit };

    // Initial size sync to backend.
    void commands.ptyResize(sessionId, term.cols, term.rows);

    // Re-fit once the document fonts have loaded so that Menlo's true cell
    // height is used rather than the fallback monospace metrics.  If fonts
    // were already ready this resolves in the same microtask queue turn.
    void document.fonts.ready.then(() => {
      if (!refs.current) return;
      syncResize(sessionId, refs.current.term, refs.current.fit);
    });

    return () => {
      detachTerminal(sessionId);
      term.dispose();
      refs.current = null;
    };
  }, [sessionId]);

  // When this terminal becomes active, refit (the host has been hidden,
  // so its dimensions may have lagged) and re-sync the backend size.
  // ResizeObserver on the host catches both window resizes and layout
  // changes (e.g. sidebar collapse) more reliably than a window "resize"
  // listener; it fires whenever the host's box size actually changes.
  useEffect(() => {
    if (!active) return;
    const r = refs.current;
    if (!r) return;

    // Re-fit after the display:none → block transition has been laid out.
    const rafId = requestAnimationFrame(() => {
      if (!refs.current) return;
      syncResize(sessionId, refs.current.term, refs.current.fit);
    });
    r.term.focus();

    const ro = new ResizeObserver(() => {
      if (!refs.current) return;
      syncResize(sessionId, refs.current.term, refs.current.fit);
    });
    ro.observe(hostRef.current as Element);

    return () => {
      cancelAnimationFrame(rafId);
      ro.disconnect();
    };
  }, [active, sessionId]);

  return (
    <div ref={hostRef} className="terminal-host" style={{ display: active ? "block" : "none" }} />
  );
}
