import { mockIPC } from "@tauri-apps/api/mocks";

type Handler = (args: any) => unknown | Promise<unknown>;

/// Declarative Tauri command mock. Throws on any unmocked command so that
/// adding a new commands.foo() in production fails loudly in tests until
/// the test (or the harness baseline) handles it.
export function mockCommands(map: Record<string, Handler>) {
  mockIPC(
    (cmd, args) => {
      const h = map[cmd];
      if (!h) throw new Error(`unmocked command: ${cmd}`);
      return h(args);
    },
    { shouldMockEvents: true },
  );
}

/// Fire a Tauri event into any listener registered via the bindings'
/// events.* helpers. Call after listen() has been awaited.
export async function emitTauriEvent(event: string, payload: unknown) {
  const { emit } = await import("@tauri-apps/api/event");
  await emit(event, payload);
}
