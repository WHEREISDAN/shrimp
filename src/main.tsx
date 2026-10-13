import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app/App";
import { setupI18n } from "./i18n";
import { loadDevBridge } from "./lib/devBridge";
import "./app/App.css";
import "./app/Pages.css";
import "./app/History.css";

loadDevBridge();
setupI18n();

const root = document.getElementById("root");
if (!root) {
  throw new Error("Missing #root element");
}

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
