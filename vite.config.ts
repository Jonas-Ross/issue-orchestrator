import { defineConfig } from "vite";

// Vite serves the frontend; Tauri starts its own webview pointing at devUrl.
// The strict port + envPrefix bits are Tauri's recommended defaults.
export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
});
