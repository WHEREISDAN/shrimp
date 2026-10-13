/**
 * Dev-only AI debugging bridge (tauri-plugin-mcp). Every webview entry calls
 * this so agents can drive each window. The condition is replaced at build
 * time, so production bundles drop the dynamic import entirely.
 */
export function loadDevBridge(): void {
  if (import.meta.env.VITE_MCP_BRIDGE === "1") {
    void import("tauri-plugin-mcp").then(({ setupPluginListeners }) => setupPluginListeners());
  }
}
