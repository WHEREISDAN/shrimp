import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Mirrors `commands::app::AppInfo` in src-tauri. */
export interface AppInfo {
  name: string;
  version: string;
  os: string;
  arch: string;
}

/** Mirrors `dictation::DictationState`. */
export type DictationState =
  { phase: "idle" } | { phase: "recording"; mode: "hold" | "locked" } | { phase: "processing" };

/** Mirrors `dictation::gestures::BarControl`. */
export type BarControl = "start" | "stop" | "cancel" | "confirm";

/** Mirrors `notify::NoticeAction`. */
export type NoticeAction =
  | "copyLastTranscript"
  | "openAccessibilitySettings"
  | "openMicrophoneSettings"
  | "openKeyboardSettings"
  | "retryModelDownload"
  | "openAiSettings";

/** Mirrors `notify::Notice`. */
export interface Notice {
  id: number;
  level: "info" | "warning" | "error";
  message: string;
  action: NoticeAction | null;
}

/** Mirrors `stt::ModelState`, flattened into `ModelStatus`. */
export type ModelState =
  | { state: "missing" }
  | { state: "downloading"; received: number; total: number }
  | { state: "downloaded" }
  | { state: "active" }
  | { state: "failed"; message: string };

/** Mirrors `stt::ModelStatus`. */
export type ModelStatus = {
  id: string;
  label: string;
  sizeBytes: number;
  isDefault: boolean;
} & ModelState;

/** Mirrors `platform::PermissionStatus`. */
export type PermissionStatus = "granted" | "denied" | "notDetermined";

/** Mirrors `platform::Permission`. */
export type Permission = "microphone" | "accessibility";

/** Mirrors `commands::permissions::PermissionsInfo`. */
export interface PermissionsInfo {
  microphone: PermissionStatus;
  accessibility: PermissionStatus;
}

/** Mirrors `commands::dictation::ShortcutInfo`. */
export interface ShortcutInfo {
  action: string;
  name: string;
  shortcut: string;
}

/** Mirrors `stt::SttEngineId`. */
export type SttEngineId = "local-whisper" | "openai" | "groq";

/** Mirrors `llm::LlmProviderId`. */
export type LlmProviderId = "anthropic" | "openai" | "groq" | "ollama";

/** Mirrors `secrets::KeyAccount`: the keychain entry a provider's key lives in. */
export type KeyAccount = "anthropic" | "openai" | "groq";

/** Mirrors `edit::CleanupLevel`. */
export type CleanupLevel = "none" | "light" | "medium";

/** Mirrors `edit::Style`. */
export type Style = "formal" | "casual" | "veryCasual" | "excited";

/** Mirrors `edit::AppCategory`. */
export type AppCategory = "personal" | "work" | "email" | "other";

/** Mirrors `settings::Settings`. */
export interface Settings {
  version: number;
  stt: { engine: SttEngineId; openaiModel: string; groqModel: string };
  ai: {
    provider: LlmProviderId;
    cleanup: CleanupLevel;
    anthropicModel: string;
    openaiModel: string;
    groqModel: string;
    /** Empty means Shrimp picks an installed model. */
    ollamaModel: string;
    ollamaUrl: string;
  };
  styles: Record<AppCategory, Style>;
}

/** A JSON merge patch for `update_settings`: any subset of the settings. */
export interface SettingsPatch {
  stt?: Partial<Settings["stt"]>;
  ai?: Partial<Settings["ai"]>;
  styles?: Partial<Settings["styles"]>;
}

/** Mirrors `llm::catalog::ModelOption`. */
export interface ModelOption {
  id: string;
  label: string;
  note: string;
}

/** Mirrors `llm::catalog::ProviderInfo`. */
export interface ProviderInfo {
  id: LlmProviderId;
  label: string;
  models: ModelOption[];
  defaultModel: string;
  key: KeyAccount | null;
}

/** Mirrors `llm::catalog::SttEngineInfo`. */
export interface SttEngineInfo {
  id: SttEngineId;
  label: string;
  models: ModelOption[];
  defaultModel: string;
  key: KeyAccount | null;
}

/** Mirrors `llm::catalog::AiOptions`. */
export interface AiOptions {
  providers: ProviderInfo[];
  sttEngines: SttEngineInfo[];
}

/** Mirrors `commands::ai::ApiKeys`: which keys are stored (never the keys). */
export type ApiKeys = Record<KeyAccount, boolean>;

/** Mirrors `commands::ai::AiTestResult`. */
export interface AiTestResult {
  ok: boolean;
  latencyMs: number;
  input: string;
  output: string;
  message: string | null;
}

/** Mirrors `storage::EntrySource`. */
export type EntrySource = "manual" | "learned" | "imported";

/** Mirrors `storage::DictionaryEntry`. */
export interface DictionaryEntry {
  id: number;
  term: string;
  misspelling: string | null;
  source: EntrySource;
  pinned: boolean;
  uses: number;
  createdAt: number;
}

/** Mirrors `storage::DictionaryInput`. */
export interface DictionaryInput {
  term: string;
  misspelling: string | null;
}

/** Mirrors `storage::SkippedRow`. */
export interface SkippedRow {
  line: number;
  reason: string;
}

/** Mirrors `storage::dictionary::ImportPreview`. */
export interface DictionaryImportPreview {
  added: DictionaryInput[];
  duplicates: DictionaryInput[];
  skipped: SkippedRow[];
}

/** Mirrors `storage::Snippet`. Expansions are a markdown subset. */
export interface Snippet {
  id: number;
  trigger: string;
  expansion: string;
  createdAt: number;
  updatedAt: number;
}

/** Mirrors `storage::SnippetInput`. */
export interface SnippetInput {
  trigger: string;
  expansion: string;
}

/** Mirrors `storage::SnippetSaved`. */
export interface SnippetSaved {
  snippet: Snippet;
  warning: string | null;
}

/** Mirrors `storage::snippets::ImportPreview`. */
export interface SnippetImportPreview {
  added: SnippetInput[];
  duplicates: SnippetInput[];
  skipped: SkippedRow[];
}

/** Mirrors `commands::CommandError`: what a rejected command throws. */
export interface CommandError {
  code: string;
  message: string;
}

/** The user-facing message of a rejected command. */
export function errorMessage(error: unknown): string {
  if (typeof error === "object" && error !== null && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}

export const getAppInfo = (): Promise<AppInfo> => invoke<AppInfo>("app_info");

export const getDictationState = (): Promise<DictationState> =>
  invoke<DictationState>("get_dictation_state");

export const flowBarControl = (control: BarControl): Promise<void> =>
  invoke("flow_bar_control", { control });

export const flowBarReady = (): Promise<void> => invoke("flow_bar_ready");

export const flowBarLayout = (width: number, height: number): Promise<void> =>
  invoke("flow_bar_layout", { width, height });

export const runNoticeAction = (action: NoticeAction): Promise<void> =>
  invoke("run_notice_action", { action });

export const getShortcuts = (): Promise<ShortcutInfo[]> => invoke<ShortcutInfo[]>("get_shortcuts");

export const getModels = (): Promise<ModelStatus[]> => invoke<ModelStatus[]>("get_models");

export const downloadDefaultModel = (): Promise<void> => invoke("download_default_model");

export const getPermissions = (): Promise<PermissionsInfo> =>
  invoke<PermissionsInfo>("get_permissions");

export const openPermissionSettings = (permission: Permission): Promise<void> =>
  invoke("open_permission_settings", { permission });

export const getSettings = (): Promise<Settings> => invoke<Settings>("get_settings");

export const updateSettings = (patch: SettingsPatch): Promise<Settings> =>
  invoke<Settings>("update_settings", { patch });

export const getAiOptions = (): Promise<AiOptions> => invoke<AiOptions>("get_ai_options");

export const getApiKeys = (): Promise<ApiKeys> => invoke<ApiKeys>("get_api_keys");

export const setApiKey = (account: KeyAccount, key: string): Promise<ApiKeys> =>
  invoke<ApiKeys>("set_api_key", { account, key });

export const deleteApiKey = (account: KeyAccount): Promise<ApiKeys> =>
  invoke<ApiKeys>("delete_api_key", { account });

export const listOllamaModels = (): Promise<string[]> => invoke<string[]>("list_ollama_models");

export const testAiProvider = (): Promise<AiTestResult> => invoke<AiTestResult>("test_ai_provider");

export const listDictionary = (): Promise<DictionaryEntry[]> =>
  invoke<DictionaryEntry[]>("list_dictionary");

export const addDictionaryEntry = (input: DictionaryInput): Promise<DictionaryEntry> =>
  invoke<DictionaryEntry>("add_dictionary_entry", { input });

export const updateDictionaryEntry = (
  id: number,
  input: DictionaryInput,
): Promise<DictionaryEntry> => invoke<DictionaryEntry>("update_dictionary_entry", { id, input });

export const setDictionaryPinned = (id: number, pinned: boolean): Promise<DictionaryEntry> =>
  invoke<DictionaryEntry>("set_dictionary_pinned", { id, pinned });

export const deleteDictionaryEntries = (ids: number[]): Promise<number> =>
  invoke<number>("delete_dictionary_entries", { ids });

export const previewDictionaryImport = (csv: string): Promise<DictionaryImportPreview> =>
  invoke<DictionaryImportPreview>("preview_dictionary_import", { csv });

export const importDictionary = (csv: string): Promise<number> =>
  invoke<number>("import_dictionary", { csv });

export const listSnippets = (): Promise<Snippet[]> => invoke<Snippet[]>("list_snippets");

export const addSnippet = (input: SnippetInput): Promise<SnippetSaved> =>
  invoke<SnippetSaved>("add_snippet", { input });

export const updateSnippet = (id: number, input: SnippetInput): Promise<SnippetSaved> =>
  invoke<SnippetSaved>("update_snippet", { id, input });

export const deleteSnippets = (ids: number[]): Promise<number> =>
  invoke<number>("delete_snippets", { ids });

export const previewSnippetImport = (json: string): Promise<SnippetImportPreview> =>
  invoke<SnippetImportPreview>("preview_snippet_import", { json });

export const importSnippets = (json: string): Promise<number> =>
  invoke<number>("import_snippets", { json });

/** Event names emitted by src-tauri/src/app/events.rs. */
export const EVENTS = {
  state: "dictation://state",
  level: "dictation://level",
  notice: "notify://notice",
  models: "models://status",
  navigate: "app://navigate",
  settings: "settings://changed",
} as const;

export function onEvent<T>(name: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(name, (event) => handler(event.payload));
}
