import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app/App";
import "./app/App.css";

// Dev-only AI debugging bridge (tauri-plugin-mcp). The condition is replaced at
// build time, so production bundles drop the dynamic import entirely.
if (import.meta.env.VITE_MCP_BRIDGE === "1") {
  void import("tauri-plugin-mcp").then(({ setupPluginListeners }) => setupPluginListeners());
}

const root = document.getElementById("root");
if (!root) {
  throw new Error("Missing #root element");
}

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
