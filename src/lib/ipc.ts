import { invoke } from "@tauri-apps/api/core";

/** Mirrors `commands::app::AppInfo` in src-tauri. */
export interface AppInfo {
  name: string;
  version: string;
  os: string;
  arch: string;
}

export function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("app_info");
}
