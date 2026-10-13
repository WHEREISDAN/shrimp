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
  | "retryModelDownload";

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

/** Event names emitted by src-tauri/src/app/events.rs. */
export const EVENTS = {
  state: "dictation://state",
  level: "dictation://level",
  notice: "notify://notice",
  models: "models://status",
  navigate: "app://navigate",
} as const;

export function onEvent<T>(name: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(name, (event) => handler(event.payload));
}
