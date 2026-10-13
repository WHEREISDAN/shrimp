import { mockIPC } from "@tauri-apps/api/mocks";
import { fireEvent } from "@testing-library/react";
import type {
  AiOptions,
  DictionaryEntry,
  Languages,
  MicrophoneList,
  ModelStatus,
  Settings,
  ShortcutList,
  Snippet,
} from "../lib/ipc";

export type Args = Record<string, unknown>;
export type Handler = (args: Args) => unknown;

export interface Call {
  cmd: string;
  args: Args;
}

/** Answers each command with its handler and records every call. Unknown commands reject. */
export function mockBackend(handlers: Record<string, Handler>) {
  const calls: Call[] = [];
  mockIPC(
    (cmd, payload) => {
      const args = (payload ?? {}) as Args;
      calls.push({ cmd, args });
      const handler = handlers[cmd];
      if (!handler) {
        throw new Error(`unexpected command ${cmd}`);
      }
      return handler(args);
    },
    { shouldMockEvents: true },
  );
  return {
    calls,
    /** The arguments of every call to `cmd`. */
    argsOf: (cmd: string) => calls.filter((call) => call.cmd === cmd).map((call) => call.args),
  };
}

/** Picks a file in the page's hidden file input. */
export function chooseFile(file: File) {
  const input = document.querySelector<HTMLInputElement>("input[type=file]");
  if (!input) {
    throw new Error("no file input");
  }
  fireEvent.change(input, { target: { files: [file] } });
}

export const entry = (overrides: Partial<DictionaryEntry> & { id: number }): DictionaryEntry => ({
  term: `word${overrides.id}`,
  misspelling: null,
  source: "manual",
  pinned: false,
  uses: 0,
  createdAt: 0,
  ...overrides,
});

/** A dictionary backend that remembers what was added and deleted. */
export function mockDictionary(initial: DictionaryEntry[], extra: Record<string, Handler> = {}) {
  let entries = initial;
  return mockBackend({
    list_dictionary: () => entries,
    add_dictionary_entry: (args) => {
      const input = args.input as { term: string; misspelling: string | null };
      entries = [...entries, entry({ id: 100, ...input })];
      return entries[entries.length - 1];
    },
    update_dictionary_entry: () => entries[0],
    set_dictionary_pinned: () => entries[0],
    delete_dictionary_entries: (args) => {
      const ids = args.ids as number[];
      entries = entries.filter((candidate) => !ids.includes(candidate.id));
      return ids.length;
    },
    ...extra,
  });
}

export const snippet = (overrides: Partial<Snippet> & { id: number }): Snippet => ({
  trigger: `trigger${overrides.id}`,
  expansion: "text",
  createdAt: 0,
  updatedAt: 0,
  ...overrides,
});

export const SETTINGS: Settings = {
  version: 1,
  stt: {
    engine: "local-whisper",
    localModel: "large-v3-turbo-q5_0",
    openaiModel: "whisper-1",
    groqModel: "whisper-large-v3-turbo",
  },
  ai: {
    provider: "anthropic",
    cleanup: "light",
    anthropicModel: "claude-haiku",
    openaiModel: "gpt-mini",
    groqModel: "llama-fast",
    ollamaModel: "",
    ollamaUrl: "http://localhost:11434",
  },
  styles: { personal: "veryCasual", work: "formal", email: "formal", other: "casual" },
  privacy: { contextAwareness: true },
  experimental: { commandMode: false, commandModeIntroSeen: false, pressEnterCommand: false },
  shortcuts: { bindings: {} },
  notices: {
    commandModeOffCount: 0,
    commandModeOffDay: "",
    bluetoothAdviceCount: 0,
    bluetoothAdviceAt: 0,
    bluetoothAdviceOff: false,
    languageCardDay: "",
    languageCardOff: false,
  },
  transforms: {
    enabled: false,
    polish: {
      rules: { concise: true, clarity: true, reorder: true, structure: true, tone: true },
      instructions: [],
    },
    promptEngineer: { prompt: "Turn the text into a clear prompt." },
    custom: [],
    autoApply: null,
  },
  vibeCoding: { fileTagging: true },
  dictionary: { autoLearn: true },
  microphone: { selected: null, ranking: [] },
  sounds: { dictation: true, notifications: true },
  media: { mute: false, pause: false },
  languages: { autoDetect: false, selected: ["en-US"], active: null },
};

export const MODELS: ModelStatus[] = [
  {
    id: "tiny-q5_1",
    label: "Whisper Tiny",
    sizeBytes: 32_152_673,
    isDefault: false,
    inUse: false,
    state: "missing",
  },
  {
    id: "small-q5_1",
    label: "Whisper Small",
    sizeBytes: 190_085_487,
    isDefault: false,
    inUse: false,
    state: "downloaded",
  },
  {
    id: "large-v3-turbo-q5_0",
    label: "Whisper Large v3 Turbo",
    sizeBytes: 574_041_195,
    isDefault: true,
    inUse: true,
    state: "active",
  },
];

export const LANGUAGES: Languages = {
  options: [
    { id: "zh-Hans", name: "Chinese (Simplified)", group: "zh" },
    { id: "zh-Hant", name: "Chinese (Traditional)", group: "zh" },
    { id: "en-CA", name: "English (Canada)", group: "en" },
    { id: "en-GB", name: "English (UK)", group: "en" },
    { id: "en-US", name: "English (US)", group: "en" },
    { id: "fr", name: "French", group: null },
    { id: "de", name: "German", group: "de" },
    { id: "de-CH", name: "Swiss German", group: "de" },
  ],
  chosen: { autoDetect: false, selected: ["en-US"], active: null },
};

export const MICROPHONES: MicrophoneList = {
  selected: null,
  active: "usb",
  devices: [
    {
      id: "usb",
      name: "Yeti",
      kind: "wired",
      connected: true,
      unavailable: null,
      rank: 0,
    },
    {
      id: "builtin",
      name: "MacBook Pro Microphone",
      kind: "builtIn",
      connected: true,
      unavailable: "Laptop lid is closed",
      rank: 1,
    },
    {
      id: "pods",
      name: "Dan's AirPods",
      kind: "bluetooth",
      connected: false,
      unavailable: null,
      rank: 2,
    },
    {
      id: "hole",
      name: "BlackHole 2ch",
      kind: "virtual",
      connected: true,
      unavailable: null,
      rank: null,
    },
  ],
};

export const SHORTCUTS: ShortcutList = {
  rows: [
    { action: "pushToTalk", name: "Push to talk", shortcuts: ["fn"], max: 4, required: true },
    {
      action: "handsFree",
      name: "Hands-free mode",
      shortcuts: ["fn Space"],
      max: 4,
      required: false,
    },
    { action: "commandMode", name: "Command Mode", shortcuts: ["fn ⌃"], max: 4, required: true },
    {
      action: "pasteLast",
      name: "Paste last transcript",
      shortcuts: ["⌃⌘V"],
      max: 4,
      required: true,
    },
    { action: "cancel", name: "Cancel dictation", shortcuts: ["Esc"], max: 1, required: true },
    { action: "pressEnter", name: "Press Enter", shortcuts: [], max: 4, required: false },
  ],
  warning: null,
  secureInput: null,
};

export const AI_OPTIONS: AiOptions = {
  providers: [
    {
      id: "anthropic",
      label: "Anthropic",
      models: [
        { id: "claude-haiku", label: "Claude Haiku", note: "Fast and cheap." },
        { id: "claude-sonnet", label: "Claude Sonnet", note: "More careful." },
      ],
      defaultModel: "claude-haiku",
      key: "anthropic",
    },
    {
      id: "openai",
      label: "OpenAI",
      models: [{ id: "gpt-mini", label: "GPT Mini", note: "" }],
      defaultModel: "gpt-mini",
      key: "openai",
    },
    {
      id: "groq",
      label: "Groq",
      models: [{ id: "llama-fast", label: "Llama Fast", note: "" }],
      defaultModel: "llama-fast",
      key: "groq",
    },
    { id: "ollama", label: "Ollama", models: [], defaultModel: "", key: null },
  ],
  sttEngines: [
    { id: "local-whisper", label: "Whisper on this Mac", models: [], defaultModel: "", key: null },
    {
      id: "openai",
      label: "OpenAI Whisper",
      models: [{ id: "whisper-1", label: "Whisper 1", note: "" }],
      defaultModel: "whisper-1",
      key: "openai",
    },
    {
      id: "groq",
      label: "Groq Whisper",
      models: [{ id: "whisper-large-v3-turbo", label: "Large v3 Turbo", note: "Fastest." }],
      defaultModel: "whisper-large-v3-turbo",
      key: "groq",
    },
  ],
};
