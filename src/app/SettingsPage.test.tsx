import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { ApiKeys, Settings, SettingsPatch } from "../lib/ipc";
import { SettingsPage } from "./SettingsPage";
import {
  AI_OPTIONS,
  LANGUAGES,
  MICROPHONES,
  MODELS,
  mockBackend,
  SETTINGS,
  SHORTCUTS,
  type Args,
} from "./testBackend";

function backend(
  extra: Record<string, (args: Args) => unknown> = {},
  initial: Settings = SETTINGS,
) {
  let settings = initial;
  let keys: ApiKeys = { anthropic: false, openai: true, groq: false };
  return mockBackend({
    get_settings: () => settings,
    get_ai_options: () => AI_OPTIONS,
    get_api_keys: () => keys,
    get_shortcuts: () => SHORTCUTS,
    get_models: () => MODELS,
    get_microphones: () => MICROPHONES,
    get_languages: () => LANGUAGES,
    set_languages: (args) => ({
      autoDetect: args.autoDetect,
      selected: args.selected,
      active: null,
    }),
    select_microphone: () => MICROPHONES,
    rank_microphone: () => MICROPHONES,
    forget_microphone: () => MICROPHONES,
    start_mic_preview: () => null,
    stop_mic_preview: () => null,
    download_model: () => null,
    delete_model: () => null,
    update_settings: (args) => {
      const patch = args.patch as SettingsPatch;
      settings = {
        ...settings,
        stt: { ...settings.stt, ...patch.stt },
        ai: { ...settings.ai, ...patch.ai },
        privacy: { ...settings.privacy, ...patch.privacy },
        experimental: { ...settings.experimental, ...patch.experimental },
        vibeCoding: { ...settings.vibeCoding, ...patch.vibeCoding },
        sounds: { ...settings.sounds, ...patch.sounds },
        media: { ...settings.media, ...patch.media },
        flowBar: { ...settings.flowBar, ...patch.flowBar },
        notifications: { ...settings.notifications, ...patch.notifications },
      };
      return settings;
    },
    set_api_key: (args) => {
      keys = { ...keys, [args.account as string]: true };
      return keys;
    },
    delete_api_key: (args) => {
      keys = { ...keys, [args.account as string]: false };
      return keys;
    },
    ...extra,
  });
}

describe("SettingsPage AI provider", () => {
  it("shows the provider, its model and the model note", async () => {
    backend();
    render(<SettingsPage />);

    expect(await screen.findByRole("combobox", { name: "Provider" })).toHaveValue("anthropic");
    expect(screen.getByRole("combobox", { name: "Model" })).toHaveValue("claude-haiku");
    expect(screen.getByText("Fast and cheap.")).toBeInTheDocument();
    expect(
      screen.getByText("Your dictation text is sent to Anthropic for cleanup, with your own key."),
    ).toBeInTheDocument();
  });

  it("saves the chosen model for the provider", async () => {
    const { argsOf } = backend();
    render(<SettingsPage />);
    fireEvent.change(await screen.findByRole("combobox", { name: "Model" }), {
      target: { value: "claude-sonnet" },
    });
    await waitFor(() =>
      expect(argsOf("update_settings")).toEqual([
        { patch: { ai: { anthropicModel: "claude-sonnet" } } },
      ]),
    );
    await waitFor(() => expect(screen.getByText("More careful.")).toBeInTheDocument());
  });

  it("saves a key, clears the field and never shows the key", async () => {
    const { argsOf } = backend();
    render(<SettingsPage />);
    expect(await screen.findByText("No key yet")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Remove key" })).not.toBeInTheDocument();

    const input = screen.getByLabelText("Anthropic API key");
    expect(input).toHaveAttribute("type", "password");
    expect(input).toHaveAttribute("autocomplete", "off");
    fireEvent.change(input, { target: { value: "sk-ant-secret" } });
    fireEvent.click(screen.getByRole("button", { name: "Save key" }));

    expect(await screen.findByText("Key saved in your keychain")).toBeInTheDocument();
    expect(argsOf("set_api_key")).toEqual([{ account: "anthropic", key: "sk-ant-secret" }]);
    expect(input).toHaveValue("");
    expect(screen.queryByDisplayValue("sk-ant-secret")).not.toBeInTheDocument();
  });

  it("removes a saved key", async () => {
    const { argsOf } = backend();
    render(<SettingsPage />);
    fireEvent.change(await screen.findByRole("combobox", { name: "Provider" }), {
      target: { value: "openai" },
    });
    expect(await screen.findByText("Key saved in your keychain")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Remove key" }));
    expect(await screen.findByText("No key yet")).toBeInTheDocument();
    expect(argsOf("delete_api_key")).toEqual([{ account: "openai" }]);
  });

  it("shows a rejected key", async () => {
    backend({
      set_api_key: () => {
        throw { code: "keychain", message: "The keychain is locked" };
      },
    });
    render(<SettingsPage />);
    fireEvent.change(await screen.findByLabelText("Anthropic API key"), { target: { value: "k" } });
    fireEvent.click(screen.getByRole("button", { name: "Save key" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("The keychain is locked");
  });

  it("lists Ollama models when Ollama is chosen", async () => {
    const { argsOf } = backend({ list_ollama_models: () => ["llama3.2", "qwen3"] });
    render(<SettingsPage />);
    fireEvent.change(await screen.findByRole("combobox", { name: "Provider" }), {
      target: { value: "ollama" },
    });

    const models = await screen.findByRole("combobox", { name: "Ollama model" });
    await waitFor(() =>
      expect(
        within(models)
          .getAllByRole("option")
          .map((option) => option.textContent),
      ).toEqual(["Automatic (recommended)", "llama3.2", "qwen3"]),
    );
    expect(models).toHaveValue("");
    expect(screen.getByLabelText("Address")).toHaveValue("http://localhost:11434");
    expect(screen.getByText("Ollama runs on this Mac; nothing leaves it.")).toBeInTheDocument();
    expect(screen.queryByRole("combobox", { name: "Model" })).not.toBeInTheDocument();
    expect(argsOf("update_settings")).toEqual([{ patch: { ai: { provider: "ollama" } } }]);

    fireEvent.change(models, { target: { value: "qwen3" } });
    await waitFor(() =>
      expect(argsOf("update_settings")[1]).toEqual({ patch: { ai: { ollamaModel: "qwen3" } } }),
    );
  });

  it("saves the Ollama address on blur and on Enter", async () => {
    const { argsOf } = backend(
      { list_ollama_models: () => [] },
      { ...SETTINGS, ai: { ...SETTINGS.ai, provider: "ollama" } },
    );
    render(<SettingsPage />);
    const address = await screen.findByLabelText("Address");

    fireEvent.change(address, { target: { value: "http://studio:11434" } });
    fireEvent.blur(address);
    await waitFor(() =>
      expect(argsOf("update_settings")).toEqual([
        { patch: { ai: { ollamaUrl: "http://studio:11434" } } },
      ]),
    );

    const saved = await screen.findByDisplayValue("http://studio:11434");
    fireEvent.blur(saved);
    expect(argsOf("update_settings")).toHaveLength(1);

    saved.focus();
    fireEvent.change(saved, { target: { value: "http://nas:11434" } });
    fireEvent.keyDown(saved, { key: "Enter" });
    await waitFor(() =>
      expect(argsOf("update_settings")[1]).toEqual({
        patch: { ai: { ollamaUrl: "http://nas:11434" } },
      }),
    );
  });

  it("offers a retry when Ollama is not running", async () => {
    let running = false;
    const { argsOf } = backend(
      {
        list_ollama_models: () => {
          if (!running) {
            throw { code: "ollama", message: "connection refused" };
          }
          return ["llama3.2"];
        },
      },
      { ...SETTINGS, ai: { ...SETTINGS.ai, provider: "ollama" } },
    );
    render(<SettingsPage />);

    expect(
      await screen.findByText("Ollama isn't running at http://localhost:11434."),
    ).toBeInTheDocument();
    running = true;
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(await screen.findByRole("option", { name: "llama3.2" })).toBeInTheDocument();
    expect(screen.queryByText(/isn't running/)).not.toBeInTheDocument();
    expect(argsOf("list_ollama_models")).toHaveLength(2);
  });

  it("tests the provider and reports the result", async () => {
    let result: unknown = {
      ok: true,
      latencyMs: 412,
      input: "um hi",
      output: "Hi.",
      message: null,
    };
    backend({ test_ai_provider: () => result });
    render(<SettingsPage />);

    fireEvent.click(await screen.findByRole("button", { name: "Test" }));
    expect(await screen.findByText("Cleaned up in 412 ms: “Hi.”")).toBeInTheDocument();

    result = { ok: false, latencyMs: 0, input: "", output: "", message: "Invalid API key" };
    fireEvent.click(screen.getByRole("button", { name: "Test" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Invalid API key");

    result = { ok: false, latencyMs: 0, input: "", output: "", message: null };
    fireEvent.click(screen.getByRole("button", { name: "Test" }));
    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("Didn't work."));
  });

  it("says Testing… while the test runs", async () => {
    backend({ test_ai_provider: () => new Promise(() => undefined) });
    render(<SettingsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Test" }));
    expect(screen.getByText("Testing…")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Test" })).toBeDisabled();
  });
});

describe("SettingsPage speech engine", () => {
  it("switches the engine and shows the cloud model and key", async () => {
    const { argsOf } = backend();
    render(<SettingsPage />);

    const group = await screen.findByRole("radiogroup", { name: "Speech engine" });
    expect(within(group).getByRole("radio", { name: "Whisper on this Mac" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    expect(screen.queryByRole("combobox", { name: "Speech model" })).not.toBeInTheDocument();
    expect(
      screen.getByText(/Whisper on this Mac is private and works offline/),
    ).toBeInTheDocument();

    fireEvent.click(within(group).getByRole("radio", { name: "Groq Whisper" }));
    expect(await screen.findByRole("combobox", { name: "Speech model" })).toHaveValue(
      "whisper-large-v3-turbo",
    );
    expect(argsOf("update_settings")).toEqual([{ patch: { stt: { engine: "groq" } } }]);
    expect(screen.getByText("Fastest.")).toBeInTheDocument();
    // Keys are named after the company, shared with the AI provider.
    expect(screen.getByLabelText("Groq API key")).toBeInTheDocument();
  });

  it("saves the speech model", async () => {
    const { argsOf } = backend({}, { ...SETTINGS, stt: { ...SETTINGS.stt, engine: "openai" } });
    render(<SettingsPage />);
    fireEvent.change(await screen.findByRole("combobox", { name: "Speech model" }), {
      target: { value: "whisper-1" },
    });
    await waitFor(() =>
      expect(argsOf("update_settings")).toEqual([{ patch: { stt: { openaiModel: "whisper-1" } } }]),
    );
  });
});

describe("SettingsPage data and privacy", () => {
  it("turns context awareness off", async () => {
    const { argsOf } = backend();
    render(<SettingsPage />);
    const toggle = await screen.findByRole("switch", { name: "Context awareness" });
    expect(toggle).toBeChecked();
    fireEvent.click(toggle);
    await waitFor(() =>
      expect(argsOf("update_settings")).toEqual([
        { patch: { privacy: { contextAwareness: false } } },
      ]),
    );
    await waitFor(() => expect(toggle).not.toBeChecked());
  });
});

describe("SettingsPage notifications", () => {
  it("mutes a category", async () => {
    const { argsOf } = backend();
    render(<SettingsPage />);
    const milestones = await screen.findByRole("switch", { name: "Milestones" });
    expect(milestones).toBeChecked();
    fireEvent.click(milestones);
    await waitFor(() =>
      expect(argsOf("update_settings")).toEqual([
        { patch: { notifications: { milestones: false } } },
      ]),
    );
    await waitFor(() => expect(milestones).not.toBeChecked());
  });
});

describe("SettingsPage experimental", () => {
  it("introduces Command Mode once when it is turned on", async () => {
    const { argsOf } = backend();
    render(<SettingsPage />);
    fireEvent.click(await screen.findByRole("switch", { name: "Command Mode" }));
    const dialog = await screen.findByRole("dialog", { name: "Using Command Mode (beta)" });
    expect(dialog).toHaveTextContent("fn ⌃");
    fireEvent.click(within(dialog).getByRole("button", { name: "Disable" }));
    await waitFor(() =>
      expect(argsOf("update_settings")).toEqual([
        { patch: { experimental: { commandMode: true } } },
        { patch: { experimental: { commandModeIntroSeen: true, commandMode: false } } },
      ]),
    );
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("turns the press enter voice command on", async () => {
    const { argsOf } = backend();
    render(<SettingsPage />);
    fireEvent.click(await screen.findByRole("switch", { name: "“Press enter” voice command" }));
    await waitFor(() =>
      expect(argsOf("update_settings")).toEqual([
        { patch: { experimental: { pressEnterCommand: true } } },
      ]),
    );
  });
});

describe("SettingsPage Whisper models", () => {
  it("lists the models with size and state, and switches, downloads and deletes", async () => {
    const { argsOf } = backend();
    render(<SettingsPage />);
    const list = await screen.findByRole("list", { name: "Whisper models" });
    expect(await within(list).findByText(/574 MB · Recommended · In use/)).toBeInTheDocument();
    expect(within(list).getByText(/32 MB · Not downloaded/)).toBeInTheDocument();
    expect(within(list).getByText(/190 MB · Downloaded/)).toBeInTheDocument();
    expect(
      within(list).queryByRole("button", { name: "Delete Whisper Large v3 Turbo" }),
      "the model in use can't be deleted",
    ).toBeNull();
    fireEvent.click(within(list).getByRole("button", { name: "Download" }));
    fireEvent.click(within(list).getByRole("button", { name: "Delete Whisper Small" }));
    fireEvent.click(within(list).getByRole("radio", { name: /Whisper Tiny/ }));
    await waitFor(() => {
      expect(argsOf("download_model")).toEqual([{ id: "tiny-q5_1" }]);
      expect(argsOf("delete_model")).toEqual([{ id: "small-q5_1" }]);
      expect(argsOf("update_settings")).toEqual([{ patch: { stt: { localModel: "tiny-q5_1" } } }]);
    });
  });
});

describe("SettingsPage shortcuts", () => {
  it("shows push to talk and opens the Shortcuts dialog", async () => {
    backend();
    const opened: string[] = [];
    render(<SettingsPage onOpenShortcuts={() => opened.push("shortcuts")} />);
    const section = await screen.findByRole("region", { name: "Shortcuts" });
    expect(await within(section).findByText("fn")).toBeInTheDocument();
    fireEvent.click(within(section).getByRole("button", { name: "Change" }));
    expect(opened).toEqual(["shortcuts"]);
  });
});

describe("SettingsPage vibe coding", () => {
  it("turns file tagging off and explains variable recognition", async () => {
    const { argsOf } = backend();
    render(<SettingsPage />);
    fireEvent.click(await screen.findByRole("switch", { name: "File tagging in chat" }));
    await waitFor(() =>
      expect(argsOf("update_settings")).toEqual([
        { patch: { vibeCoding: { fileTagging: false } } },
      ]),
    );
    expect(screen.getByText(/Toggle Screen Reader Accessibility Mode/)).toBeInTheDocument();
  });
});

describe("SettingsPage microphone", () => {
  it("lists Auto-detect and the microphones with their state, and ranks and forgets them", async () => {
    const { argsOf } = backend();
    render(<SettingsPage />);
    const list = await screen.findByRole("list", { name: "Microphones" });
    expect(within(list).getByText("Yeti")).toBeInTheDocument();
    expect(within(list).getByText(/In use/)).toBeInTheDocument();
    expect(within(list).getByText("Laptop lid is closed")).toBeInTheDocument();
    expect(within(list).getByText(/Not connected/)).toBeInTheDocument();
    expect(within(list).queryByText("BlackHole 2ch"), "virtual devices are hidden").toBeNull();
    expect(screen.getByRole("radio", { name: /Auto-detect/ })).toBeChecked();
    expect(
      screen.getByRole("radio", { name: /MacBook Pro Microphone/ }),
      "unavailable devices can't be chosen",
    ).toBeDisabled();

    fireEvent.click(screen.getByRole("checkbox", { name: /Show other devices/ }));
    expect(within(list).getByText("BlackHole 2ch")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("radio", { name: /Yeti/ }));
    fireEvent.click(screen.getByRole("button", { name: "Move MacBook Pro Microphone up" }));
    fireEvent.click(screen.getByRole("button", { name: "Forget Dan's AirPods" }));
    await waitFor(() => {
      expect(argsOf("select_microphone")).toEqual([{ id: "usb" }]);
      expect(argsOf("rank_microphone")).toEqual([{ id: "builtin", up: true }]);
      expect(argsOf("forget_microphone")).toEqual([{ id: "pods" }]);
    });
    expect(argsOf("start_mic_preview")).toHaveLength(1);
    expect(screen.getByRole("meter", { name: "Microphone level" })).toBeInTheDocument();
  });

  it("walks through Find the right mic for you", async () => {
    backend();
    render(<SettingsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Find the right mic for you" }));
    fireEvent.click(screen.getByRole("button", { name: "A quiet room" }));
    fireEvent.click(screen.getByRole("button", { name: "A desktop Mac (mini, iMac, Studio)" }));
    fireEvent.click(screen.getByRole("button", { name: "No" }));
    expect(screen.getByText("A USB microphone on your desk")).toBeInTheDocument();
  });
});

describe("SettingsPage sound", () => {
  it("turns sounds off and keeps muting and pausing exclusive", async () => {
    const { argsOf } = backend();
    render(<SettingsPage />);
    fireEvent.click(await screen.findByRole("switch", { name: "Notification sounds" }));
    fireEvent.click(screen.getByRole("switch", { name: "Mute audio while dictating" }));
    await waitFor(() => expect(argsOf("update_settings")).toHaveLength(2));
    fireEvent.click(screen.getByRole("switch", { name: "Pause music while dictating" }));
    await waitFor(() =>
      expect(argsOf("update_settings")).toEqual([
        { patch: { sounds: { notifications: false } } },
        { patch: { media: { mute: true, pause: false } } },
        { patch: { media: { pause: true, mute: false } } },
      ]),
    );
  });
});

describe("SettingsPage languages", () => {
  it("searches, keeps one variant per language and saves", async () => {
    const { argsOf } = backend();
    render(<SettingsPage />);
    const section = await screen.findByRole("region", { name: "Languages" });
    expect(await within(section).findByText("English (US)")).toBeInTheDocument();
    fireEvent.click(within(section).getByRole("button", { name: "Change" }));
    const dialog = screen.getByRole("dialog", { name: "Languages" });
    fireEvent.change(within(dialog).getByRole("searchbox", { name: "Search languages" }), {
      target: { value: "engl" },
    });
    expect(within(dialog).queryByRole("checkbox", { name: "French" })).toBeNull();
    fireEvent.click(within(dialog).getByRole("checkbox", { name: "English (UK)" }));
    expect(within(dialog).getByRole("checkbox", { name: "English (US)" })).not.toBeChecked();
    fireEvent.change(within(dialog).getByRole("searchbox", { name: "Search languages" }), {
      target: { value: "" },
    });
    fireEvent.click(within(dialog).getByRole("checkbox", { name: "French" }));
    fireEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(argsOf("set_languages")).toEqual([{ autoDetect: false, selected: ["en-GB", "fr"] }]),
    );
  });

  it("refuses to save with no language and auto-detect off", async () => {
    const { argsOf } = backend();
    render(<SettingsPage />);
    const section = await screen.findByRole("region", { name: "Languages" });
    fireEvent.click(await within(section).findByRole("button", { name: "Change" }));
    const dialog = screen.getByRole("dialog", { name: "Languages" });
    fireEvent.click(within(dialog).getByRole("checkbox", { name: "English (US)" }));
    fireEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    expect(within(dialog).getByRole("alert")).toHaveTextContent(
      "Please select at least one language or turn on auto-detect",
    );
    expect(argsOf("set_languages")).toEqual([]);
    fireEvent.click(within(dialog).getByRole("switch", { name: "Auto-detect" }));
    fireEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(argsOf("set_languages")).toEqual([{ autoDetect: true, selected: [] }]),
    );
  });
});

describe("SettingsPage Flow Bar", () => {
  it("turns off showing the bar at all times", async () => {
    const { argsOf } = backend();
    render(<SettingsPage />);
    fireEvent.click(await screen.findByRole("switch", { name: "Show Flow Bar at all times" }));
    await waitFor(() =>
      expect(argsOf("update_settings")).toEqual([{ patch: { flowBar: { alwaysShow: false } } }]),
    );
  });
});
