import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { OnboardingStep, PermissionsInfo, Settings } from "../../lib/ipc";
import { App } from "../App";
import {
  INSIGHTS,
  LANGUAGES,
  MICROPHONES,
  MODELS,
  mockBackend,
  SETTINGS,
  SHORTCUTS,
  VOICE_PROFILE,
} from "../testBackend";

function backend(step: OnboardingStep, permissions: PermissionsInfo, completed = false) {
  let settings: Settings = {
    ...SETTINGS,
    onboarding: { ...SETTINGS.onboarding, completed, step },
  };
  const state = { permissions };
  const mock = mockBackend({
    get_settings: () => settings,
    update_settings: (args) => {
      const patch = args.patch as {
        onboarding?: Partial<Settings["onboarding"]>;
        ai?: Partial<Settings["ai"]>;
      };
      settings = {
        ...settings,
        onboarding: { ...settings.onboarding, ...patch.onboarding },
        ai: { ...settings.ai, ...patch.ai },
      };
      return settings;
    },
    get_app_health: () => ({ crash: null, data: null, storage: null }),
    get_permissions: () => state.permissions,
    open_permission_settings: () => null,
    get_shortcuts: () => SHORTCUTS,
    get_microphones: () => MICROPHONES,
    start_mic_preview: () => null,
    stop_mic_preview: () => null,
    get_languages: () => LANGUAGES,
    get_models: () => MODELS,
    get_api_keys: () => ({ anthropic: false, openai: false, groq: false }),
    list_history: () => ({ entries: [], disabled: false }),
    run_notice_action: () => null,
    app_info: () => ({
      name: "Shrimp",
      version: "0.1.0",
      os: "macos",
      arch: "aarch64",
      userName: "Dan",
    }),
    get_insights: () => INSIGHTS,
    get_voice_profile: () => VOICE_PROFILE,
  });
  return { ...mock, state, settings: () => settings };
}

const GRANTED: PermissionsInfo = { microphone: "granted", accessibility: "granted" };

const title = (name: string) => screen.findByRole("heading", { name, level: 1 });
const next = (name = "Continue") => fireEvent.click(screen.getByRole("button", { name }));

describe("Onboarding", () => {
  it("walks through every step, saving each one", async () => {
    const { argsOf, settings } = backend("welcome", GRANTED);
    render(<App />);
    expect(await title("Welcome to Shrimp")).toHaveFocus();
    next("Get started");
    expect(await title("Let Shrimp hear you and type for you")).toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole("button", { name: "Continue" })).toBeEnabled());
    next();
    expect(await title("Test your microphone")).toBeInTheDocument();
    expect(await screen.findByRole("radio", { name: /Auto-detect/ })).toBeInTheDocument();
    next();
    expect(await title("Your dictation shortcut")).toBeInTheDocument();
    expect(
      screen.getByText("Hold fn and speak, then let go to insert the text."),
    ).toBeInTheDocument();
    next();
    expect(await title("Which languages do you speak?")).toBeInTheDocument();
    next();
    expect(await title("Set up the speech engine")).toBeInTheDocument();
    fireEvent.click(await screen.findByRole("radio", { name: /^Ollama/ }));
    next();
    expect(await title("Try it")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Skip" })).toBeInTheDocument();
    next();
    expect(await title("You're all set")).toBeInTheDocument();
    next("Start using Shrimp");
    expect(await screen.findByRole("navigation", { name: "Main" })).toBeInTheDocument();
    expect(settings().onboarding.completed).toBe(true);
    expect(settings().ai.provider).toBe("ollama");
    const steps = argsOf("update_settings")
      .map((args) => (args.patch as { onboarding?: { step?: string } }).onboarding?.step)
      .filter(Boolean);
    expect(steps).toEqual([
      "permissions",
      "microphone",
      "shortcut",
      "languages",
      "engine",
      "practice",
      "done",
      "done",
    ]);
  });

  it("waits for both permissions and sees a grant as it happens", async () => {
    const { state } = backend("permissions", {
      microphone: "notDetermined",
      accessibility: "denied",
    });
    render(<App />);
    await title("Let Shrimp hear you and type for you");
    expect(await screen.findByRole("button", { name: "Allow" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Continue" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Open System Settings" })).toBeInTheDocument();
    state.permissions = GRANTED;
    await act(() => new Promise((resolve) => setTimeout(resolve, 1100)));
    await waitFor(() => expect(screen.getByRole("button", { name: "Continue" })).toBeEnabled());
    expect(screen.getAllByText(/Allowed/)).toHaveLength(2);
  });

  it("resumes where it stopped", async () => {
    backend("languages", GRANTED);
    render(<App />);
    expect(await title("Which languages do you speak?")).toBeInTheDocument();
  });

  it("asks again for a permission lost after an update", async () => {
    backend("done", { microphone: "granted", accessibility: "denied" }, true);
    render(<App />);
    expect(await title("Grant Shrimp's permissions again")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Later" }));
    expect(await screen.findByRole("navigation", { name: "Main" })).toBeInTheDocument();
  });
});
