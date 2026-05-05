import { defineConfig } from "vitest/config";
import preact from "@preact/preset-vite";

export default defineConfig({
  plugins: [preact()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
    css: false,
    pool: "threads",
    isolate: true,
    coverage: {
      provider: "v8",
      reporter: ["text", "html"],
      include: ["src/**/*.{ts,tsx}"],
      exclude: [
        "src/lib/bindings.ts",
        "src/components/TerminalView.tsx",
        "src/components/IssuePicker.tsx",
        "src/main.tsx",
        "src/**/*.test.{ts,tsx}",
        "src/test/**",
      ],
    },
  },
});
