import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { loadDevBridge } from "../lib/devBridge";
import { FlowBar } from "./FlowBar";
import "./FlowBar.css";

loadDevBridge();

const root = document.getElementById("root");
if (!root) {
  throw new Error("Missing #root element");
}

createRoot(root).render(
  <StrictMode>
    <FlowBar />
  </StrictMode>,
);
