import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { EVENTS, type ModelStatus, type PermissionsInfo, type ShortcutInfo } from "../lib/ipc";
import { App } from "./App";
import { AI_OPTIONS, SETTINGS } from "./testBackend";

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
      case "list_dictionary":
        return [];
      case "get_settings":
        return SETTINGS;
      case "get_ai_options":
        return AI_OPTIONS;
      case "get_api_keys":
        return { anthropic: false, openai: false, groq: false };
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

  it("opens on Home without asking for anything the pages need", async () => {
    const commands: string[] = [];
    const answer = backend([model({})]);
    mockIPC(
      (cmd) => {
        commands.push(cmd);
        return answer(cmd);
      },
      { shouldMockEvents: true },
    );
    render(<App />);
    await screen.findByTestId("app-info");

    expect(screen.getByRole("button", { name: "Home" })).toHaveAttribute("aria-current", "page");
    expect(
      screen.getAllByRole("button").filter((button) => button.hasAttribute("aria-current")),
    ).toHaveLength(1);
    expect(new Set(commands.filter((cmd) => !cmd.startsWith("plugin:")))).toEqual(
      new Set(["app_info", "get_models", "get_permissions", "get_shortcuts"]),
    );
  });

  it("shows the Dictionary page when its sidebar item is clicked", async () => {
    mockIPC(backend([model({})]), { shouldMockEvents: true });
    render(<App />);
    await screen.findByTestId("app-info");

    fireEvent.click(screen.getByRole("button", { name: "Dictionary" }));

    expect(await screen.findByRole("heading", { name: "Dictionary" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Dictionary" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(screen.getByRole("button", { name: "Home" })).not.toHaveAttribute("aria-current");
    expect(screen.queryByRole("heading", { name: "Shortcuts" })).not.toBeInTheDocument();
    expect(
      await screen.findByText(
        "No words yet. Add names, jargon and spellings Shrimp should get right.",
      ),
    ).toBeInTheDocument();
  });

  it("offers every page from the sidebar", async () => {
    mockIPC(backend([model({})]), { shouldMockEvents: true });
    render(<App />);
    await screen.findByTestId("app-info");

    for (const name of ["Style", "Settings"]) {
      fireEvent.click(screen.getByRole("button", { name }));
      expect(await screen.findByRole("heading", { level: 1, name })).toBeInTheDocument();
    }
    fireEvent.click(screen.getByRole("button", { name: "Snippets" }));
    expect(screen.getByRole("heading", { level: 1, name: "Snippets" })).toBeInTheDocument();
  });

  describe("navigation from the tray", () => {
    const scrollIntoView = vi.fn<(options?: ScrollIntoViewOptions) => void>();
    Object.defineProperty(Element.prototype, "scrollIntoView", {
      configurable: true,
      value: scrollIntoView,
    });
    afterEach(() => scrollIntoView.mockClear());

    it("opens Settings", async () => {
      mockIPC(backend([model({})]), { shouldMockEvents: true });
      render(<App />);
      await screen.findByTestId("app-info");

      await act(async () => emit(EVENTS.navigate, "settings"));

      expect(
        await screen.findByRole("heading", { level: 1, name: "Settings" }),
      ).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Settings" })).toHaveAttribute(
        "aria-current",
        "page",
      );
    });

    it("returns to Home and scrolls to Shortcuts from another page", async () => {
      mockIPC(backend([model({})]), { shouldMockEvents: true });
      render(<App />);
      await screen.findByTestId("app-info");
      fireEvent.click(screen.getByRole("button", { name: "Dictionary" }));
      await screen.findByRole("heading", { name: "Dictionary" });

      await act(async () => emit(EVENTS.navigate, "shortcuts"));

      const heading = await screen.findByRole("heading", { name: "Shortcuts" });
      await waitFor(() => expect(scrollIntoView).toHaveBeenCalledWith({ block: "start" }));
      expect(heading).toHaveFocus();
      expect(screen.getByRole("button", { name: "Home" })).toHaveAttribute("aria-current", "page");
    });

    it("scrolls to Shortcuts when Home is already open", async () => {
      mockIPC(backend([model({})]), { shouldMockEvents: true });
      render(<App />);
      await screen.findByTestId("app-info");
      expect(scrollIntoView).not.toHaveBeenCalled();

      await act(async () => emit(EVENTS.navigate, "shortcuts"));

      await waitFor(() => expect(scrollIntoView).toHaveBeenCalledTimes(1));
    });
  });
});
