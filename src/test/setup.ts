import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/preact";
import { afterEach, beforeEach } from "vitest";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";

// Tauri's runtime sniffs window.__TAURI_INTERNALS__ on first import. Install a
// no-op IPC + event-mock baseline before every test. Tests override per-test
// with mockCommands(...) from src/test/tauri-mock.ts.
beforeEach(() => {
  mockIPC(() => undefined, { shouldMockEvents: true });
});

afterEach(() => {
  cleanup();
  clearMocks();
  localStorage.clear();
});
