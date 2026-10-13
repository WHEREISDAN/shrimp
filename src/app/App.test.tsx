import { mockIPC } from "@tauri-apps/api/mocks";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { ModelStatus, PermissionsInfo, ShortcutInfo } from "../lib/ipc";
import { App } from "./App";

const model = (overrides: Partial<ModelStatus>): ModelStatus =>
  ({
    id: "large-v3-turbo-q5_0",
    label: "Whisper Large v3 Turbo",
    sizeBytes: 574_041_195,
    isDefault: true,
    state: "active",
    ...overrides,
  }) as ModelStatus;

function backend(models: ModelStatus[]) {
  const permissions: PermissionsInfo = { microphone: "notDetermined", accessibility: "granted" };
  const shortcuts: ShortcutInfo[] = [
    { action: "pushToTalk", name: "Push to talk", shortcut: "fn" },
    { action: "pasteLast", name: "Paste last transcript", shortcut: "⌃⌘V" },
  ];
  return (cmd: string) => {
    switch (cmd) {
      case "app_info":
        return { name: "Shrimp", version: "0.1.0", os: "macos", arch: "aarch64" };
      case "get_models":
        return models;
      case "get_permissions":
        return permissions;
      case "get_shortcuts":
        return shortcuts;
      default:
        throw new Error(`unexpected command ${cmd}`);
    }
  };
}

describe("App", () => {
  it("shows the app info returned by the backend", async () => {
    mockIPC(backend([model({})]), { shouldMockEvents: true });
    render(<App />);
    expect(await screen.findByTestId("app-info")).toHaveTextContent("Shrimp 0.1.0 · macos/aarch64");
  });

  it("shows an error when the backend call fails", async () => {
    mockIPC(
      () => {
        throw new Error("backend unavailable");
      },
      { shouldMockEvents: true },
    );
    render(<App />);
    expect(await screen.findByRole("alert")).toHaveTextContent("backend unavailable");
  });

  it("shows model download progress", async () => {
    mockIPC(backend([model({ state: "downloading", received: 287_020_598, total: 574_041_195 })]), {
      shouldMockEvents: true,
    });
    render(<App />);
    expect(await screen.findByTestId("model-progress")).toHaveTextContent(
      "Downloading 50% (287 of 574 MB)",
    );
  });

  it("lists permissions and shortcuts", async () => {
    mockIPC(backend([model({})]), { shouldMockEvents: true });
    render(<App />);
    expect(await screen.findByText("Not asked yet")).toBeInTheDocument();
    expect(await screen.findByRole("button", { name: "Allow…" })).toBeInTheDocument();
    expect(await screen.findByText("⌃⌘V")).toBeInTheDocument();
    expect(await screen.findByText(/Ready\. Dictation runs entirely/)).toBeInTheDocument();
  });
});
