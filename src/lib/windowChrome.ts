import { invoke } from "@tauri-apps/api/core";
import { IS_WINDOWS } from "./platform";

/** Mirrors `app::window::WindowMaterial`. */
export type WindowMaterial = "vibrancy" | "mica" | "none";

/**
 * Tells the stylesheet which platform it runs on and which native material the
 * window shows, so the sidebar can leave it clear. macOS always has vibrancy;
 * Windows 11 has Mica and Windows 10 none, which only Rust can tell.
 */
export function setUpWindowChrome(root: HTMLElement = document.documentElement) {
  root.dataset.platform = IS_WINDOWS ? "windows" : "macos";
  root.dataset.material = IS_WINDOWS ? "none" : "vibrancy";
  if (IS_WINDOWS) {
    invoke<WindowMaterial>("window_material")
      .then((material) => {
        root.dataset.material = material;
      })
      .catch(() => undefined);
  }
}
