import { invoke } from "@tauri-apps/api/core";
import { IS_WINDOWS } from "./platform";

/** Mirrors `app::window::WindowMaterial`. */
export type WindowMaterial = "vibrancy" | "mica" | "none";

const CLICKABLE =
  "a, button, input, select, textarea, label, summary, [contenteditable]:not([contenteditable=false]), [tabindex]:not([tabindex='-1']), [role=button], [role=link], [role=menuitem], [role=tab], [role=checkbox], [role=radio], [role=switch], [role=option]";

/**
 * Whether an event's path runs through the title area: an element with
 * `data-tauri-drag-region` (its whole subtree for "deep"), with no control in
 * between, as Tauri decides where a drag starts.
 */
export function inTitleArea(path: EventTarget[]): boolean {
  for (const node of path) {
    if (!(node instanceof HTMLElement)) {
      continue;
    }
    const region = node.getAttribute("data-tauri-drag-region");
    if (region === null) {
      if (node.matches(CLICKABLE)) {
        return false;
      }
      continue;
    }
    if (region === "false") {
      return false;
    }
    return region === "deep" || node === path[0];
  }
  return false;
}

function keepsBrowserMenu(target: EventTarget | null): boolean {
  if (target instanceof HTMLElement && target.closest("input, textarea, [contenteditable]")) {
    return true;
  }
  return Boolean(window.getSelection()?.toString());
}

/**
 * Windows: a right click on the title area opens the window menu, as on a
 * native title bar. Elsewhere WebView2's browser menu (Back, Refresh, Print…)
 * only opens for text fields and selected text, where it offers Cut, Copy and
 * Paste.
 */
export function handleContextMenu(event: MouseEvent) {
  if (inTitleArea(event.composedPath())) {
    event.preventDefault();
    invoke("show_window_menu").catch(() => undefined);
  } else if (!keepsBrowserMenu(event.target)) {
    event.preventDefault();
  }
}

/**
 * Tells the stylesheet which platform it runs on and which native material the
 * window shows, so the sidebar can leave it clear. macOS always has vibrancy;
 * Windows 11 has Mica and Windows 10 none, which only Rust can tell.
 */
export function setUpWindowChrome(root: HTMLElement = document.documentElement) {
  root.dataset.platform = IS_WINDOWS ? "windows" : "macos";
  root.dataset.material = IS_WINDOWS ? "none" : "vibrancy";
  if (IS_WINDOWS) {
    document.addEventListener("contextmenu", handleContextMenu);
    invoke<WindowMaterial>("window_material")
      .then((material) => {
        root.dataset.material = material;
      })
      .catch(() => undefined);
  }
}
