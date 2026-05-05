import { mockCommands, emitTauriEvent } from "../../test/tauri-mock";
import { createPtyStream } from "../pty-stream";

// Minimal stand-in for an xterm Terminal. The pty stream only calls .write.
function fakeTerm() {
  return { write: vi.fn() } as unknown as import("@xterm/xterm").Terminal & {
    write: ReturnType<typeof vi.fn>;
  };
}

async function tick() {
  // emit() resolves before listen() callbacks fire on the next microtask;
  // one queueMicrotask cycle is enough to flush them in jsdom.
  await new Promise<void>((r) => queueMicrotask(r));
}

describe("pty-stream demuxer", () => {
  beforeEach(() => {
    // Tauri's event mocking needs the IPC baseline; no commands are called
    // by pty-stream itself, but starting listeners requires the IPC hook.
    mockCommands({});
  });

  it("buffers chunks that arrive before a terminal attaches", async () => {
    const { startPtyStream, attachTerminal } = createPtyStream();
    startPtyStream();

    await emitTauriEvent("pty-data", { sessionId: "s1", chunk: "hello " });
    await emitTauriEvent("pty-data", { sessionId: "s1", chunk: "world" });
    await tick();

    const term = fakeTerm();
    attachTerminal("s1", term);

    expect(term.write).toHaveBeenCalledTimes(2);
    expect(term.write).toHaveBeenNthCalledWith(1, "hello ");
    expect(term.write).toHaveBeenNthCalledWith(2, "world");
  });

  it("writes directly to the terminal once attached", async () => {
    const { startPtyStream, attachTerminal } = createPtyStream();
    startPtyStream();
    const term = fakeTerm();
    attachTerminal("s1", term);

    await emitTauriEvent("pty-data", { sessionId: "s1", chunk: "live" });
    await tick();

    expect(term.write).toHaveBeenCalledWith("live");
  });

  it("scopes buffers per sessionId", async () => {
    const { startPtyStream, attachTerminal } = createPtyStream();
    startPtyStream();

    await emitTauriEvent("pty-data", { sessionId: "s1", chunk: "for-1" });
    await emitTauriEvent("pty-data", { sessionId: "s2", chunk: "for-2" });
    await tick();

    const term1 = fakeTerm();
    attachTerminal("s1", term1);
    expect(term1.write).toHaveBeenCalledTimes(1);
    expect(term1.write).toHaveBeenCalledWith("for-1");

    const term2 = fakeTerm();
    attachTerminal("s2", term2);
    expect(term2.write).toHaveBeenCalledWith("for-2");
  });

  it("detachTerminal clears any pending buffer for that session", async () => {
    const { startPtyStream, attachTerminal, detachTerminal } = createPtyStream();
    startPtyStream();

    await emitTauriEvent("pty-data", { sessionId: "s1", chunk: "queued" });
    await tick();
    detachTerminal("s1");

    const term = fakeTerm();
    attachTerminal("s1", term);

    expect(term.write).not.toHaveBeenCalled();
  });

  it("startPtyStream is idempotent (only listens once)", async () => {
    const { startPtyStream, attachTerminal } = createPtyStream();
    startPtyStream();
    startPtyStream(); // second call is a no-op

    const term = fakeTerm();
    attachTerminal("s1", term);
    await emitTauriEvent("pty-data", { sessionId: "s1", chunk: "once" });
    await tick();

    // If start were not idempotent, we'd see 2 writes for one emit.
    expect(term.write).toHaveBeenCalledTimes(1);
  });

  it("isolated streams don't share state", async () => {
    const a = createPtyStream();
    const b = createPtyStream();
    a.startPtyStream();
    b.startPtyStream();

    const termA = fakeTerm();
    a.attachTerminal("s1", termA);
    const termB = fakeTerm();
    b.attachTerminal("s1", termB);

    await emitTauriEvent("pty-data", { sessionId: "s1", chunk: "shared-event" });
    await tick();

    // Each stream has its own listener registered via Tauri's listen, and
    // both fire on the same event — but the fact that each stream has its
    // own terminal map proves the closures are isolated.
    expect(termA.write).toHaveBeenCalledWith("shared-event");
    expect(termB.write).toHaveBeenCalledWith("shared-event");
  });
});
