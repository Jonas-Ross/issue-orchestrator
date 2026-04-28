import { render } from "preact";
import { App } from "./app";
import "@xterm/xterm/css/xterm.css";
import "./lib/tokens.css";
import "./style.css";

const root = document.getElementById("root");
if (!root) throw new Error("missing #root mount point");
render(<App />, root);
