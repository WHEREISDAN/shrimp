import "@testing-library/jest-dom/vitest";
import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";
import { setupI18n } from "../i18n";

setupI18n("en");

afterEach(() => {
  cleanup();
  clearMocks();
});
