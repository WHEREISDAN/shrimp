import { emit } from "@tauri-apps/api/event";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { EVENTS, type AppHealth, type Settings } from "../lib/ipc";
import { App } from "./App";
import { ErrorScreen } from "./ErrorScreen";
import { SettingsPage } from "./SettingsPage";
import {
  AI_OPTIONS,
  INSIGHTS,
  LANGUAGES,
  MICROPHONES,
  MODELS,
  mockBackend,
  SETTINGS,
  SHORTCUTS,
  VOICE_PROFILE,
} from "./testBackend";

const HEALTHY: AppHealth = { crash: null, data: null, storage: null, misplaced: false };

function backend(health: AppHealth = HEALTHY, extra: Record<string, () => unknown> = {}) {
  let settings: Settings = SETTINGS;
  return mockBackend({
    app_info: () => ({
      name: "Shrimp",
      version: "0.1.0",
      os: "macos",
      arch: "aarch64",
      userName: "Dan",
    }),
    get_app_health: () => health,
    get_settings: () => settings,
    update_settings: (args) => {
      const patch = args.patch as { app?: Partial<Settings["app"]> };
      settings = { ...settings, app: { ...settings.app, ...patch.app } };
      return settings;
    },
    get_models: () => MODELS,
    get_permissions: () => ({ microphone: "granted", accessibility: "granted" }),
    get_shortcuts: () => SHORTCUTS,
    list_history: () => ({ entries: [], disabled: false }),
    get_insights: () => INSIGHTS,
    get_voice_profile: () => VOICE_PROFILE,
    get_ai_options: () => AI_OPTIONS,
    get_api_keys: () => ({ anthropic: false, openai: false, groq: false }),
    get_languages: () => LANGUAGES,
    get_microphones: () => MICROPHONES,
    start_mic_preview: () => null,
    stop_mic_preview: () => null,
    list_dictionary: () => [],
    reset_app: () => null,
    dismiss_crash: () => HEALTHY,
    check_storage: () => HEALTHY,
    open_storage_settings: () => null,
    repair_data: () => null,
    move_to_applications: () => null,
    quit_app: () => null,
    ...extra,
  });
}

const current = () =>
  screen.getAllByRole("button").find((button) => button.getAttribute("aria-current") === "page");

describe("App navigation", () => {
  it("puts Settings and Help at the bottom of the sidebar", async () => {
    backend();
    render(<App />);
    const nav = await screen.findByRole("navigation", { name: "Main" });
    const names = within(nav)
      .getAllByRole("button")
      .map((button) => button.textContent);
    expect(names).toEqual([
      "Home",
      "Insights",
      "Dictionary",
      "Snippets",
      "Style",
      "Transforms",
      "Settings",
      "Help",
    ]);
  });

  it("goes back and forward with ⌘[ and ⌘], and through the sidebar with ⌥↑/⌥↓", async () => {
    backend();
    render(<App />);
    await screen.findByRole("heading", { name: /Dan$/ });
    fireEvent.keyDown(window, { key: "ArrowDown", altKey: true });
    expect(current()).toHaveTextContent("Insights");
    fireEvent.click(screen.getByRole("button", { name: "Help" }));
    expect(await screen.findByRole("heading", { name: "Help", level: 1 })).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "[", metaKey: true });
    expect(current()).toHaveTextContent("Insights");
    fireEvent.keyDown(window, { key: "[", metaKey: true });
    expect(current()).toHaveTextContent("Home");
    fireEvent.keyDown(window, { key: "]", metaKey: true });
    expect(current()).toHaveTextContent("Insights");
  });

  it("opens the AI provider page from a notice", async () => {
    backend();
    render(<App />);
    await screen.findByRole("heading", { name: /Dan$/ });
    await act(() => emit(EVENTS.navigate, "ai"));
    expect(await screen.findByRole("tab", { name: "AI provider" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
  });

  it("shows an error screen instead of the app when something is wrong", async () => {
    backend({ crash: { details: null }, data: null, storage: null, misplaced: false });
    render(<App />);
    expect(
      await screen.findByRole("heading", { name: "Shrimp quit unexpectedly" }),
    ).toBeInTheDocument();
    expect(screen.queryByRole("navigation", { name: "Main" })).not.toBeInTheDocument();
  });
});

describe("Settings pages", () => {
  it("lists the pages and moves with the arrows and ⌥↑/⌥↓", async () => {
    backend();
    render(<SettingsPage />);
    const tabs = await screen.findByRole("tablist", { name: "Settings pages" });
    expect(
      within(tabs)
        .getAllByRole("tab")
        .map((tab) => tab.textContent),
    ).toEqual([
      "General",
      "System",
      "Data and privacy",
      "Speech engine",
      "AI provider",
      "Experimental",
      "Vibe coding",
    ]);
    const general = screen.getByRole("tab", { name: "General" });
    expect(general).toHaveAttribute("aria-selected", "true");
    general.focus();
    fireEvent.keyDown(general, { key: "ArrowDown" });
    expect(screen.getByRole("tab", { name: "System" })).toHaveFocus();
    expect(await screen.findByRole("switch", { name: "Launch at login" })).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "ArrowDown", altKey: true });
    expect(screen.getByRole("tab", { name: "Data and privacy" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
  });

  it("handles ⌥↑/⌥↓ before the sidebar's listener, even one added earlier", async () => {
    backend();
    const handled: boolean[] = [];
    const sidebar = (event: KeyboardEvent) => handled.push(event.defaultPrevented);
    window.addEventListener("keydown", sidebar);
    try {
      render(<SettingsPage />);
      await screen.findByRole("tab", { name: "General" });
      fireEvent.keyDown(window, { key: "ArrowUp", altKey: true });
    } finally {
      window.removeEventListener("keydown", sidebar);
    }
    expect(handled).toEqual([true]);
    expect(screen.getByRole("tab", { selected: true })).toHaveTextContent("Vibe coding");
  });

  it("turns launch at login on, hides the Dock icon and resets after confirming", async () => {
    const { argsOf } = backend();
    render(<SettingsPage target={{ section: "system", count: 0 }} />);
    fireEvent.click(await screen.findByRole("switch", { name: "Launch at login" }));
    fireEvent.click(screen.getByRole("switch", { name: "Show app in Dock" }));
    await waitFor(() =>
      expect(argsOf("update_settings")).toEqual([
        { patch: { app: { launchAtLogin: true } } },
        { patch: { app: { showInDock: false } } },
      ]),
    );
    fireEvent.click(screen.getByRole("button", { name: "Reset & restart" }));
    const dialog = screen.getByRole("dialog", { name: "Reset and restart?" });
    expect(dialog).toHaveTextContent("Downloaded speech models are kept.");
    fireEvent.click(within(dialog).getByRole("button", { name: "Reset & restart" }));
    await waitFor(() => expect(argsOf("reset_app")).toEqual([{}]));
  });

  it("chooses the app language", async () => {
    const { argsOf } = backend();
    render(<SettingsPage />);
    const select = await screen.findByRole("combobox", {
      name: "Language of Shrimp's windows, menus and notices",
    });
    expect(
      within(select)
        .getAllByRole("option")
        .map((option) => option.textContent),
    ).toEqual(["Same as the Mac", "English", "Deutsch", "Español", "Italiano", "Português"]);
    fireEvent.change(select, { target: { value: "de" } });
    await waitFor(() =>
      expect(argsOf("update_settings")).toEqual([{ patch: { app: { language: "de" } } }]),
    );
  });
});

describe("ErrorScreen", () => {
  const misplaced: AppHealth = { ...HEALTHY, misplaced: true, crash: { details: null } };

  it("asks to move Shrimp to Applications before anything else", async () => {
    backend(misplaced);
    render(<App />);
    expect(
      await screen.findByRole("heading", { name: "Move Shrimp to the Applications folder" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Shrimp needs to be in the Applications folder to run and update."),
    ).toBeInTheDocument();
    expect(screen.queryByRole("navigation", { name: "Main" })).not.toBeInTheDocument();
  });

  it("moves to Applications or quits", async () => {
    const { argsOf } = backend();
    render(<ErrorScreen health={misplaced} />);
    fireEvent.click(screen.getByRole("button", { name: "Move to Applications" }));
    await waitFor(() => expect(argsOf("move_to_applications")).toEqual([{}]));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Move to Applications" })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "Quit" }));
    await waitFor(() => expect(argsOf("quit_app")).toEqual([{}]));
  });

  it("keeps the screen and says why when the move fails", async () => {
    backend(HEALTHY, {
      move_to_applications: () => {
        throw {
          code: "platform",
          message: "Shrimp couldn't be copied to /Applications: Permission denied",
        };
      },
    });
    render(<ErrorScreen health={misplaced} />);
    fireEvent.click(screen.getByRole("button", { name: "Move to Applications" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Shrimp couldn't be copied to /Applications: Permission denied",
    );
    expect(
      screen.getByRole("heading", { name: "Move Shrimp to the Applications folder" }),
    ).toBeInTheDocument();
  });

  it("offers the backup for damaged data", async () => {
    const { argsOf } = backend();
    render(
      <ErrorScreen
        health={{ crash: null, storage: null, data: { backup: "2026-10-12" }, misplaced: false }}
      />,
    );
    expect(
      screen.getByRole("heading", { name: "Some of your local data is corrupted." }),
    ).toHaveFocus();
    expect(screen.getByText("The latest backup is from October 12, 2026.")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Restore backup" }));
    await waitFor(() => expect(argsOf("repair_data")).toEqual([{ repair: "restoreBackup" }]));
    fireEvent.click(screen.getByRole("button", { name: "Start fresh" }));
    await waitFor(() => expect(argsOf("repair_data")).toHaveLength(2));
  });

  it("says the disk is full and checks again", async () => {
    const { argsOf } = backend();
    render(
      <ErrorScreen
        health={{ crash: null, data: null, storage: { freeBytes: 52_000_000 }, misplaced: false }}
      />,
    );
    expect(screen.getByRole("heading", { name: "Storage full" })).toBeInTheDocument();
    expect(screen.getByText("52 MB free")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Check again" }));
    await waitFor(() => expect(argsOf("check_storage")).toHaveLength(1));
    await waitFor(() => expect(screen.getByRole("button", { name: "Check again" })).toBeEnabled());
    fireEvent.click(screen.getByRole("button", { name: "Open Storage Settings" }));
    await waitFor(() => expect(argsOf("open_storage_settings")).toHaveLength(1));
  });

  it("continues after a crash", async () => {
    const { argsOf } = backend();
    render(
      <ErrorScreen
        health={{
          crash: { details: "panicked at x.rs" },
          data: null,
          storage: null,
          misplaced: false,
        }}
      />,
    );
    expect(screen.getByText("panicked at x.rs")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    await waitFor(() => expect(argsOf("dismiss_crash")).toEqual([{}]));
  });
});
