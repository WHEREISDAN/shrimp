import { emit } from "@tauri-apps/api/event";
import { act, fireEvent, render, screen, within } from "@testing-library/react";
import axe from "axe-core";
import { describe, expect, it } from "vitest";
import { App } from "./app/App";
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
} from "./app/testBackend";
import { FlowBar } from "./flow-bar/FlowBar";
import { EVENTS, type HistoryItem, type OnboardingStep, type Settings } from "./lib/ipc";

const HISTORY: HistoryItem[] = [
  {
    id: 1,
    createdAt: Date.now(),
    status: "inserted",
    appName: "TextEdit",
    rawText: "um ship it",
    finalText: "Ship it.",
    formatted: true,
    showRaw: false,
    language: "en",
    audioMs: 3000,
    wordCount: 2,
    error: null,
    hasAudio: true,
    retry: null,
  },
  {
    id: 2,
    createdAt: Date.now() - 1000,
    status: "failed",
    appName: "Slack",
    rawText: "",
    finalText: "",
    formatted: false,
    showRaw: false,
    language: null,
    audioMs: 6000,
    wordCount: 0,
    error: "Transcription failed",
    hasAudio: true,
    retry: "retry",
  },
];

function backend(settings: Settings = SETTINGS) {
  return mockBackend({
    app_info: () => ({
      name: "Shrimp",
      version: "0.1.0",
      os: "macos",
      arch: "aarch64",
      userName: "Dan",
    }),
    get_app_health: () => ({ crash: null, data: null, storage: null, misplaced: false }),
    get_settings: () => settings,
    update_settings: () => settings,
    get_format_apps: () => [{ id: "com.apple.Notes", name: "Notes" }],
    get_permissions: () => ({ microphone: "granted", accessibility: "granted" }),
    get_models: () => MODELS,
    get_shortcuts: () => SHORTCUTS,
    list_history: () => ({ entries: HISTORY, disabled: false }),
    get_insights: () => INSIGHTS,
    get_voice_profile: () => VOICE_PROFILE,
    get_ai_options: () => AI_OPTIONS,
    get_api_keys: () => ({ anthropic: false, openai: false, groq: false }),
    get_languages: () => LANGUAGES,
    get_microphones: () => MICROPHONES,
    start_mic_preview: () => null,
    stop_mic_preview: () => null,
    list_dictionary: () => [],
    list_snippets: () => [],
    list_ollama_models: () => [],
    get_dictation_state: () => ({ phase: "idle" }),
    flow_bar_ready: () => null,
  });
}

/** Every axe violation except the ones jsdom can't judge (contrast, layout). */
async function violations(): Promise<string[]> {
  const results = await axe.run(document.body, {
    rules: { "color-contrast": { enabled: false } },
  });
  return results.violations.map(
    (violation) =>
      `${violation.id}: ${violation.nodes.map((node) => node.target.join(" ")).join(" | ")}`,
  );
}

describe("accessibility (A11Y-01, A11Y-02)", () => {
  it("every page of the main window has named controls and valid roles", async () => {
    backend();
    render(<App />);
    const nav = await screen.findByRole("navigation", { name: "Main" });
    const pages = within(nav).getAllByRole("button");
    for (const page of pages) {
      fireEvent.click(page);
      await screen.findByRole("heading", { level: 1 });
      await act(() => new Promise((resolve) => setTimeout(resolve, 30)));
      expect({ where: page.textContent ?? "", violations: await violations() }).toEqual({
        where: page.textContent ?? "",
        violations: [],
      });
    }
  });

  it("every settings page passes", async () => {
    backend();
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "Settings" }));
    const tabs = await screen.findAllByRole("tab");
    for (const tab of tabs) {
      fireEvent.click(tab);
      await act(() => new Promise((resolve) => setTimeout(resolve, 30)));
      expect({ where: tab.textContent ?? "", violations: await violations() }).toEqual({
        where: tab.textContent ?? "",
        violations: [],
      });
    }
  });

  it("dialogs and the history menu pass", async () => {
    backend();
    render(<App />);
    fireEvent.click((await screen.findAllByRole("button", { name: "More options" }))[0]!);
    expect(await violations()).toEqual([]);
    fireEvent.click(screen.getByRole("menuitem", { name: "Delete transcript" }));
    expect(await violations()).toEqual([]);
  });

  it("each onboarding step passes", async () => {
    const steps: OnboardingStep[] = [
      "welcome",
      "permissions",
      "microphone",
      "shortcut",
      "languages",
      "engine",
      "practice",
      "done",
    ];
    for (const step of steps) {
      backend({ ...SETTINGS, onboarding: { ...SETTINGS.onboarding, completed: false, step } });
      const { unmount } = render(<App />);
      await screen.findByRole("heading", { level: 1 });
      await act(() => new Promise((resolve) => setTimeout(resolve, 30)));
      expect({ where: step, violations: await violations() }).toEqual({
        where: step,
        violations: [],
      });
      unmount();
    }
  });

  it("the Flow Bar passes resting, recording and with a notice", async () => {
    backend();
    render(<FlowBar />);
    await screen.findByRole("button", { name: "Start dictation" });
    expect(await violations()).toEqual([]);
    await act(() => emit(EVENTS.state, { phase: "recording", mode: "locked", command: false }));
    await act(() =>
      emit(EVENTS.notice, {
        id: 1,
        level: "error",
        message: "No microphone detected",
        actions: ["openSoundSettings"],
        category: "status",
      }),
    );
    expect(await violations()).toEqual([]);
  });

  it("the Flow Bar's wand, its menu and the changes view pass", async () => {
    backend({ ...SETTINGS, transforms: { ...SETTINGS.transforms, enabled: true } });
    render(<FlowBar />);
    await screen.findByRole("button", { name: "Start dictation" });
    await act(() => emit(EVENTS.flowBarHover, true));
    fireEvent.click(await screen.findByRole("button", { name: "Choose a transform" }));
    await act(() =>
      emit(EVENTS.changes, { id: 1, name: "Polish", original: "teh plan", result: "The plan." }),
    );
    expect(await violations()).toEqual([]);
  });
});
