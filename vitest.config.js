import { defineConfig } from "vitest/config";

// Deliberately separate from vite.config.js: pure logic tests only (no DOM,
// no Tauri APIs), so this never touches what actually ships in the app.
export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.js"],
  },
});
