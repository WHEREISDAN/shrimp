import { resolve } from "node:path";
import process from "node:process";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],
  // Keep Rust compiler errors visible in the `tauri dev` terminal.
  clearScreen: false,
  server: {
    // tauri.conf.json `devUrl` expects this exact port.
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    // One page per webview: the main window and the Flow Bar.
    rollupOptions: {
      input: {
        main: resolve(import.meta.dirname, "index.html"),
        "flow-bar": resolve(import.meta.dirname, "flow-bar.html"),
      },
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["src/test/setup.ts"],
    restoreMocks: true,
  },
});
