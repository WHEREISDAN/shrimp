import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Mirrors `commands::app::AppInfo` in src-tauri. */
export interface AppInfo {
  name: string;
  version: string;
  os: string;
  arch: string;
  /** The user's first name, for the greeting. */
  userName: string | null;
}

/** Mirrors `dictation::DictationState`. */
export type DictationState =
  | { phase: "idle" }
  | { phase: "recording"; mode: "hold" | "locked"; command: boolean }
  | { phase: "processing" };

/** Mirrors `dictation::gestures::BarControl`. */
export type BarControl = "start" | "stop" | "cancel" | "confirm";

/** Mirrors `notify::NoticeAction`. */
export type NoticeAction =
  | "copyLastTranscript"
  | "openAccessibilitySettings"
  | "openMicrophoneSettings"
  | "openKeyboardSettings"
  | "retryModelDownload"
  | "openAiSettings"
  | "openExperimentalSettings"
  | "viewTransformChanges"
  | "openDictionary"
  | "undoCancel"
  | "chooseMicrophone"
  | "switchMicrophone"
  | "switchToNewMicrophone"
  | "keepMicrophone"
  | "dismissBluetoothAdvice"
  | "openSoundSettings"
  | "troubleshootMicrophone"
  | "openLanguageSettings"
  | "addDetectedLanguage"
  | "dismissLanguageCard"
  | "showFlowBar"
  | "openHistory"
  | "openInsights"
  | "seeVoiceProfile"
  | "updateVoiceProfile";

/** Mirrors `notify::Notice`. */
export interface Notice {
  id: number;
  level: "info" | "warning" | "error";
  message: string;
  /** Buttons, in order. */
  actions: NoticeAction[];
  /** Which Settings › Notifications switch mutes it; status notices always show. */
  category: "status" | "suggestion" | "announcement" | "milestone";
}

/** Mirrors `dictation::Answer`: a Command Mode answer shown in the Flow Bar. */
export interface Answer {
  id: number;
  question: string;
  text: string;
}

/** Mirrors `audio::DeviceKind`. */
export type DeviceKind = "builtIn" | "wired" | "bluetooth" | "virtual";

/** Mirrors `mic::MicrophoneEntry`: one microphone in the picker. */
export interface MicrophoneEntry {
  id: string;
  name: string;
  kind: DeviceKind;
  connected: boolean;
  /** Why it can't be used now, such as "Laptop lid is closed". */
  unavailable: string | null;
  /** Its place in the ranking, for physical microphones. */
  rank: number | null;
}

/** Mirrors `mic::MicrophoneList`. */
export interface MicrophoneList {
  /** A device id, or null for Auto-detect. */
  selected: string | null;
  /** The device a dictation would record from now. */
  active: string | null;
  devices: MicrophoneEntry[];
}

/** Mirrors `lang::catalog::LanguageOption`. */
export interface LanguageOption {
  /** A Whisper code ("fr") or a variant ("en-GB"). */
  id: string;
  name: string;
  /** Variants in one group exclude each other. */
  group: string | null;
}

/** Mirrors `settings::LanguageSettings`. */
export interface LanguageChoice {
  autoDetect: boolean;
  selected: string[];
  /** One of `selected`, chosen from the Flow Bar; null detects among them. */
  active: string | null;
}

/** Mirrors `commands::languages::Languages`. */
export interface Languages {
  options: LanguageOption[];
  chosen: LanguageChoice;
}

/** Mirrors `hotkeys::Action`: everything a shortcut can trigger. */
export type ShortcutAction =
  | "pushToTalk"
  | "handsFree"
  | "commandMode"
  | "pasteLast"
  | "copyLast"
  | "cancel"
  | "pressEnter"
  | "polish"
  | "promptEngineer"
  | "viewChanges"
  | { customTransform: number };

/** Whether two actions are the same (custom transforms are objects). */
export function sameAction(a: ShortcutAction, b: ShortcutAction): boolean {
  if (typeof a === "string" || typeof b === "string") {
    return a === b;
  }
  return a.customTransform === b.customTransform;
}

/** Mirrors `commands::shortcuts::ShortcutRow`: one action in the Shortcuts dialog. */
export interface ShortcutRow {
  action: ShortcutAction;
  name: string;
  /** Each binding, formatted ("fn", "⌃⌘V", "Mouse 4"). */
  shortcuts: string[];
  max: number;
  /** Must keep at least one binding. */
  required: boolean;
}

/** Mirrors `commands::shortcuts::ShortcutList`. */
export interface ShortcutList {
  rows: ShortcutRow[];
  /** Hands-free sits inside push to talk. */
  warning: string | null;
  /** The app whose Secure Keyboard Entry blocks keyed shortcuts. */
  secureInput: string | null;
}

/** Mirrors `dictation::RecordingStatus`: a shortcut being recorded through the hooks. */
export type RecordingStatus =
  | { state: "recording"; action: ShortcutAction; preview: string | null }
  | { state: "saved"; action: ShortcutAction; shortcut: string; warning: string | null }
  | { state: "failed"; action: ShortcutAction; message: string }
  | { state: "cancelled"; action: ShortcutAction };

/** Mirrors `dictation::TransformView`: what the last transform changed. */
export interface TransformView {
  id: number;
  name: string;
  original: string;
  result: string;
}

/** Mirrors `dictation::TransformAction`: the changes view's buttons. */
export type TransformAction = "view" | "copy" | "retry" | "undo";

/** Mirrors `settings::PolishRules`. */
export interface PolishRules {
  concise: boolean;
  clarity: boolean;
  reorder: boolean;
  structure: boolean;
  tone: boolean;
}

/** Mirrors `settings::CustomTransform`. */
export interface CustomTransform {
  id: string;
  name: string;
  prompt: string;
  /** Only sent with a new transform; the backend moves it into `shortcuts.bindings`. */
  shortcut?: string | null;
  samples: string[];
}

/** Mirrors `settings::TransformSettings`. */
export interface TransformSettings {
  enabled: boolean;
  polish: { rules: PolishRules; instructions: string[] };
  promptEngineer: { prompt: string };
  custom: CustomTransform[];
  /** "polish", "promptEngineer" or a custom transform's id. */
  autoApply: string | null;
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
  /** Shrimp's recommendation. */
  isDefault: boolean;
  /** Loaded, so dictations use it. */
  inUse: boolean;
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
  /** Grows with every saved change; a smaller one is an older state. */
  revision: number;
  stt: { engine: SttEngineId; localModel: string; openaiModel: string; groqModel: string };
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
  privacy: {
    contextAwareness: boolean;
    history: HistoryStorage;
    /** Only Whisper on this Mac and Ollama run. */
    privacyMode: boolean;
  };
  experimental: { commandMode: boolean; commandModeIntroSeen: boolean; pressEnterCommand: boolean };
  /** Bindings per action ("pushToTalk", "transform:<id>"), e.g. ["Fn", "Mouse4"]; missing means the defaults. */
  shortcuts: { bindings: Record<string, string[]> };
  notices: {
    commandModeOffCount: number;
    commandModeOffDay: string;
    bluetoothAdviceCount: number;
    bluetoothAdviceAt: number;
    bluetoothAdviceOff: boolean;
    languageCardDay: string;
    languageCardOff: boolean;
    formattingTips: { list: number; email: number; backtrack: number; day: string };
  };
  transforms: TransformSettings;
  vibeCoding: { fileTagging: boolean };
  dictionary: { autoLearn: boolean };
  microphone: {
    selected: string | null;
    ranking: { id: string; name: string; kind: DeviceKind }[];
  };
  sounds: { dictation: boolean; notifications: boolean };
  /** At most one is on. */
  media: { mute: boolean; pause: boolean };
  languages: LanguageChoice;
  flowBar: {
    alwaysShow: boolean;
    /** Seconds since 1970; the resting bar is hidden until then. */
    hiddenUntil: number;
    edge: "bottom" | "left" | "right";
    /** From 0 (left or top) to 1 along the edge. */
    position: number;
  };
  /** Settings › Notifications: the categories that can be muted. */
  notifications: { suggestions: boolean; announcements: boolean; milestones: boolean };
  app: {
    /** "system", or one of the interface languages. */
    language: AppLanguage;
    launchAtLogin: boolean;
    showInDock: boolean;
  };
  onboarding: {
    completed: boolean;
    /** Where setup resumes. */
    step: OnboardingStep;
    /** The app version that last checked the permissions. */
    version: string;
    /** Permissions seen granted, so losing one is noticed. */
    granted: { microphone: boolean; accessibility: boolean };
  };
}

/** Mirrors `settings::OnboardingStep`. */
export type OnboardingStep =
  | "welcome"
  | "permissions"
  | "microphone"
  | "shortcut"
  | "languages"
  | "engine"
  | "practice"
  | "done";

/** Mirrors `settings::APP_LANGUAGES`. */
export type AppLanguage = "system" | "en" | "de" | "es" | "it" | "pt";

/** A JSON merge patch for `update_settings`: any subset of the settings. */
export interface SettingsPatch {
  stt?: Partial<Settings["stt"]>;
  ai?: Partial<Settings["ai"]>;
  styles?: Partial<Settings["styles"]>;
  privacy?: Partial<Settings["privacy"]>;
  experimental?: Partial<Settings["experimental"]>;
  vibeCoding?: Partial<Settings["vibeCoding"]>;
  dictionary?: Partial<Settings["dictionary"]>;
  sounds?: Partial<Settings["sounds"]>;
  media?: Partial<Settings["media"]>;
  flowBar?: Partial<Settings["flowBar"]>;
  notifications?: Partial<Settings["notifications"]>;
  app?: Partial<Settings["app"]>;
  onboarding?: Partial<Omit<Settings["onboarding"], "granted">>;
  /** `null` restores a default (a whole section, or Polish's rules). */
  transforms?: {
    enabled?: boolean;
    polish?: { rules?: Partial<PolishRules> | null; instructions?: string[] };
    promptEngineer?: { prompt?: string } | null;
    custom?: CustomTransform[];
    autoApply?: string | null;
  };
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

/** Mirrors `settings::HistoryStorage`: Data and privacy › Local data storage. */
export type HistoryStorage = "local" | "autoDelete" | "never";

/** Mirrors `storage::HistoryStatus`. */
export type HistoryStatus =
  "inserted" | "copied" | "transcribed" | "failed" | "empty" | "interrupted";

/** Mirrors `history::RetryKind`. */
export type RetryKind = "retry" | "recover";

/** Mirrors `history::HistoryItem`: one dictation in Home's history. */
export interface HistoryItem {
  id: number;
  /** Unix milliseconds. */
  createdAt: number;
  status: HistoryStatus;
  appName: string | null;
  rawText: string;
  finalText: string;
  /** False when AI edits were off, skipped or failed. */
  formatted: boolean;
  /** Undo AI edit is on: the row shows the raw text. */
  showRaw: boolean;
  /** A Whisper language code. */
  language: string | null;
  audioMs: number;
  wordCount: number;
  error: string | null;
  hasAudio: boolean;
  retry: RetryKind | null;
}

/** Mirrors `history::HistoryList`. */
export interface HistoryList {
  entries: HistoryItem[];
  /** "Never store data locally" is on. */
  disabled: boolean;
}

/** Mirrors `insights::DayActivity`: one active day of the heatmap. */
export interface DayActivity {
  /** YYYY-MM-DD. */
  day: string;
  words: number;
  dictations: number;
  apps: number;
  topApp: string | null;
}

/** Mirrors `insights::Insights` (STA-01). */
export interface Insights {
  wpm: number;
  totalWords: number;
  totalDictations: number;
  wordsCorrected: number;
  dictionaryFixes: number;
  /** This month so far against the same days last month. */
  month: { current: number; previous: number } | null;
  currentStreak: number;
  longestStreak: number;
  /** Since Monday. */
  weekWords: number;
  /** Active days of the last year, oldest first. */
  days: DayActivity[];
  apps: { name: string; words: number; dictations: number }[];
  appsUsed: number;
  topApp: string | null;
}

/** Mirrors `insights::profile::VoiceProfile` (STA-04). */
export interface VoiceProfile {
  archetype: string;
  description: string;
  catchphrase: string;
  mostCorrected: string;
  activeTimes: string;
  generatedAt: number;
  /** Total words when it was generated. */
  words: number;
}

/** Mirrors `insights::profile::VoiceProfileStatus`. */
export interface VoiceProfileStatus {
  profile: VoiceProfile | null;
  totalWords: number;
  storedWords: number;
  /** Total words at which the first profile, or the next update, unlocks. */
  nextAt: number;
  neededLocally: number;
}

/** Mirrors `commands::insights::VoiceProfileError`. */
export type VoiceProfileError =
  | { kind: "notEnoughStored"; stored: number; needed: number }
  | { kind: "notYet"; nextAt: number }
  | { kind: "failed"; message: string };

/** Mirrors `health::AppHealth`: what the error screens show (APP-09). */
export interface AppHealth {
  crash: { details: string | null } | null;
  data: { backup: string | null } | null;
  storage: { freeBytes: number } | null;
}

/** Mirrors `health::Repair`. */
export type Repair = "restoreBackup" | "startFresh";

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

/** The resting bar follows the cursor while dragged, and docks to the nearest edge when `ended`. */
export const flowBarDrag = (ended: boolean): Promise<void> => invoke("flow_bar_drag", { ended });

/** Mirrors `commands::dictation::FlowBarAction`: the Flow Bar's right-click menu. */
export type FlowBarAction = "pasteLast" | "history" | "settings" | "hide";

export const flowBarAction = (action: FlowBarAction): Promise<void> =>
  invoke("flow_bar_action", { action });

export const runNoticeAction = (action: NoticeAction): Promise<void> =>
  invoke("run_notice_action", { action });

export const getShortcuts = (): Promise<ShortcutList> => invoke<ShortcutList>("get_shortcuts");

/** Records a binding through the keyboard and mouse hooks; progress arrives as `EVENTS.recording`. */
export const recordShortcut = (action: ShortcutAction, index: number | null): Promise<void> =>
  invoke("record_shortcut", { action, index });

export const cancelShortcutRecording = (): Promise<void> => invoke("cancel_shortcut_recording");

export const removeShortcut = (action: ShortcutAction, index: number): Promise<void> =>
  invoke("remove_shortcut", { action, index });

export const resetShortcuts = (): Promise<void> => invoke("reset_shortcuts");

export const getModels = (): Promise<ModelStatus[]> => invoke<ModelStatus[]>("get_models");

export const getLanguages = (): Promise<Languages> => invoke<Languages>("get_languages");

/** Saves the Languages dialog; rejects when neither a language nor auto-detect is chosen. */
export const setLanguages = (autoDetect: boolean, selected: string[]): Promise<LanguageChoice> =>
  invoke<LanguageChoice>("set_languages", { autoDetect, selected });

/** The Flow Bar's language switch; null detects among the user's languages. */
export const setActiveLanguage = (id: string | null): Promise<LanguageChoice> =>
  invoke<LanguageChoice>("set_active_language", { id });

export const getMicrophones = (): Promise<MicrophoneList> =>
  invoke<MicrophoneList>("get_microphones");

/** Chooses a microphone, or Auto-detect with null. */
export const selectMicrophone = (id: string | null): Promise<MicrophoneList> =>
  invoke<MicrophoneList>("select_microphone", { id });

export const rankMicrophone = (id: string, up: boolean): Promise<MicrophoneList> =>
  invoke<MicrophoneList>("rank_microphone", { id, up });

export const forgetMicrophone = (id: string): Promise<MicrophoneList> =>
  invoke<MicrophoneList>("forget_microphone", { id });

/** Starts the live level meter; levels arrive as `EVENTS.micLevel`. */
export const startMicPreview = (): Promise<void> => invoke("start_mic_preview");

export const stopMicPreview = (): Promise<void> => invoke("stop_mic_preview");

/** Retries getting the chosen model ready after a failure. */
export const retryModel = (): Promise<void> => invoke("retry_model");

/** Downloads a model without switching to it. */
export const downloadModel = (id: string): Promise<void> => invoke("download_model", { id });

export const deleteModel = (id: string): Promise<void> => invoke("delete_model", { id });

export const getPermissions = (): Promise<PermissionsInfo> =>
  invoke<PermissionsInfo>("get_permissions");

export const openPermissionSettings = (permission: Permission): Promise<void> =>
  invoke("open_permission_settings", { permission });

export const getSettings = (): Promise<Settings> => invoke<Settings>("get_settings");

export const updateSettings = (patch: SettingsPatch): Promise<Settings> =>
  invoke<Settings>("update_settings", { patch });

export const transformAction = (action: TransformAction): Promise<void> =>
  invoke("transform_action", { action });

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

export const listHistory = (query: string): Promise<HistoryList> =>
  invoke<HistoryList>("list_history", { query });

export const copyHistory = (id: number): Promise<void> => invoke("copy_history", { id });

/** Undo AI edit (`raw`, also copies the raw text) or Redo AI edit. */
export const setHistoryRaw = (id: number, raw: boolean): Promise<HistoryItem> =>
  invoke<HistoryItem>("set_history_raw", { id, raw });

/** Retry or Recover; the entry updates through `EVENTS.history`. */
export const retryHistory = (id: number): Promise<void> => invoke("retry_history", { id });

/** The recording as WAV bytes. */
export const historyAudio = (id: number): Promise<ArrayBuffer> =>
  invoke<ArrayBuffer>("history_audio", { id });

/** Saves the recording to Downloads and returns its path. */
export const extractHistoryAudio = (id: number): Promise<string> =>
  invoke<string>("extract_history_audio", { id });

export const deleteHistory = (id: number): Promise<void> => invoke("delete_history", { id });

/** "never" needs `confirmation` to be "delete all". */
export const setHistoryStorage = (
  storage: HistoryStorage,
  confirmation: string | null = null,
): Promise<Settings> => invoke<Settings>("set_history_storage", { storage, confirmation });

export const getInsights = (): Promise<Insights> => invoke<Insights>("get_insights");

export const getVoiceProfile = (): Promise<VoiceProfileStatus> =>
  invoke<VoiceProfileStatus>("get_voice_profile");

/** Rejects with a `VoiceProfileError`; `language` is an English name such as "German". */
export const generateVoiceProfile = (language: string): Promise<VoiceProfileStatus> =>
  invoke<VoiceProfileStatus>("generate_voice_profile", { language });

/** Puts a PNG on the clipboard. */
export const copyImage = (png: Uint8Array): Promise<void> => invoke("copy_image", png);

/** Saves a PNG to Downloads as "<name> <date>.png" and returns its path. */
export const saveImage = (png: Uint8Array, name: string): Promise<string> =>
  invoke<string>("save_image", png, { headers: { "x-file-name": encodeURIComponent(name) } });

export const getAppHealth = (): Promise<AppHealth> => invoke<AppHealth>("get_app_health");

export const dismissCrash = (): Promise<AppHealth> => invoke<AppHealth>("dismiss_crash");

export const checkStorage = (): Promise<AppHealth> => invoke<AppHealth>("check_storage");

export const openStorageSettings = (): Promise<void> => invoke("open_storage_settings");

/** Repairs the database at a restart; the app restarts right away. */
export const repairData = (repair: Repair): Promise<void> => invoke("repair_data", { repair });

/** Reset & restart. */
export const resetApp = (): Promise<void> => invoke("reset_app");

/** Event names emitted by src-tauri/src/app/events.rs. */
export const EVENTS = {
  state: "dictation://state",
  level: "dictation://level",
  notice: "notify://notice",
  models: "models://status",
  navigate: "app://navigate",
  settings: "settings://changed",
  answer: "command://answer",
  changes: "transform://changes",
  recording: "shortcuts://recording",
  dismiss: "notify://dismiss",
  micLevel: "mic://level",
  microphones: "mic://devices",
  flowBarHover: "flow-bar://hover",
  history: "history://changed",
  health: "app://health",
  mainVisible: "app://main-visible",
} as const;

export function onEvent<T>(name: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(name, (event) => handler(event.payload));
}
