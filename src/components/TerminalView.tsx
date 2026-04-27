import { useEffect, useRef } from "preact/hooks";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/// Phase 0: single-session port of the original src/main.ts. Subscribes
/// to pty:data, forwards keystrokes via pty_write, resizes on window
/// resize, and spawns one PTY on mount. Phase 1 (M2) replaces this with
/// per-session terminals driven by the registry actor.
export function TerminalView() {
  const hostRef = useRef<HTMLDivElement | null>(null);

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
      void invoke("pty_write", { data });
    });

    const sendResize = () => {
      fit.fit();
      void invoke("pty_resize", { cols: term.cols, rows: term.rows });
    };
    window.addEventListener("resize", sendResize);

    let cancelled = false;
    const cleanups: Array<() => void> = [
      () => window.removeEventListener("resize", sendResize),
      () => term.dispose(),
    ];

    // Subscribe BEFORE spawning so bash's initial prompt isn't lost.
    void (async () => {
      const unlisten = await listen<string>("pty:data", (e) => {
        term.write(e.payload);
      });
      if (cancelled) {
        unlisten();
        return;
      }
      cleanups.push(unlisten);

      await invoke("pty_spawn", { cols: term.cols, rows: term.rows });
      if (cancelled) return;
      term.focus();
    })();

    return () => {
      cancelled = true;
      while (cleanups.length) cleanups.pop()?.();
    };
  }, []);

  return <div id="terminal" ref={hostRef} />;
}
