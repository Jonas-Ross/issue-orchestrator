import { TerminalView } from "./components/TerminalView";

/// Phase 0: renders the M1 single-PTY terminal. Phase 1 (M2) replaces
/// this with a tab strip + per-session terminals.
export function App() {
  return <TerminalView />;
}
