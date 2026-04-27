import type { Terminal } from "@xterm/xterm";
import { events } from "../lib/bindings";

/// Module-level demuxer for `pty:data` events. The race is: a session's
/// PTY can emit bytes before the matching <TerminalView> has mounted and
/// registered its xterm instance. We buffer chunks per sessionId until a
/// terminal attaches, then drain the buffer into it. After detach (tab
/// closed or unmounted), further chunks are silently dropped.
const terminals = new Map<string, Terminal>();
const pending = new Map<string, string[]>();

let started = false;

export function startPtyStream(): void {
  if (started) return;
  started = true;
  void events.ptyData.listen((e) => {
    const { sessionId, chunk } = e.payload;
    const term = terminals.get(sessionId);
    if (term) {
      term.write(chunk);
      return;
    }
    const buf = pending.get(sessionId) ?? [];
    buf.push(chunk);
    pending.set(sessionId, buf);
  });
}

export function attachTerminal(sessionId: string, term: Terminal): void {
  const buf = pending.get(sessionId);
  if (buf) {
    for (const chunk of buf) term.write(chunk);
    pending.delete(sessionId);
  }
  terminals.set(sessionId, term);
}

export function detachTerminal(sessionId: string): void {
  terminals.delete(sessionId);
  pending.delete(sessionId);
}
