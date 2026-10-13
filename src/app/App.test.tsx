import { mockIPC } from "@tauri-apps/api/mocks";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "./App";

describe("App", () => {
  it("shows the app info returned by the backend", async () => {
    mockIPC((cmd) => {
      if (cmd === "app_info") {
        return { name: "Shrimp", version: "0.1.0", os: "macos", arch: "aarch64" };
      }
      throw new Error(`unexpected command ${cmd}`);
    });

    render(<App />);

    expect(await screen.findByTestId("app-info")).toHaveTextContent("Shrimp 0.1.0 · macos/aarch64");
  });

  it("shows an error when the backend call fails", async () => {
    mockIPC(() => {
      throw new Error("backend unavailable");
    });

    render(<App />);

    expect(await screen.findByRole("alert")).toHaveTextContent("backend unavailable");
  });
});
