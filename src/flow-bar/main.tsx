import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { setupI18n } from "../i18n";
import { loadDevBridge } from "../lib/devBridge";
import { FlowBar } from "./FlowBar";
import "./FlowBar.css";

loadDevBridge();
setupI18n();

const root = document.getElementById("root");
if (!root) {
  throw new Error("Missing #root element");
}

createRoot(root).render(
  <StrictMode>
    <FlowBar />
  </StrictMode>,
);
