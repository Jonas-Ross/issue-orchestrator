import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "@xterm/xterm/css/xterm.css";
import "./style.css";

const term = new Terminal({
  fontFamily: "Menlo, Monaco, monospace",
  fontSize: 13,
  cursorBlink: true,
  theme: { background: "#0d0d0d", foreground: "#e6e6e6" },
});

const fit = new FitAddon();
term.loadAddon(fit);

const el = document.getElementById("terminal")!;
term.open(el);
fit.fit();

// Subscribe BEFORE spawning so we don't miss bash's initial prompt.
await listen<string>("pty:data", (e) => {
  term.write(e.payload);
});

term.onData((data) => {
  void invoke("pty_write", { data });
});

const sendResize = () => {
  fit.fit();
  void invoke("pty_resize", { cols: term.cols, rows: term.rows });
};
window.addEventListener("resize", sendResize);

await invoke("pty_spawn", { cols: term.cols, rows: term.rows });
term.focus();
