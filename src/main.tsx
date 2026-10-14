import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app/App";
import { setupI18n } from "./i18n";
import { loadDevBridge } from "./lib/devBridge";
import { reportOwnTextFields } from "./lib/ownField";
import { setUpWindowChrome } from "./lib/windowChrome";
import "./styles/globals.css";

setUpWindowChrome();
loadDevBridge();
setupI18n();
reportOwnTextFields();

const root = document.getElementById("root");
if (!root) {
  throw new Error("Missing #root element");
}

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
