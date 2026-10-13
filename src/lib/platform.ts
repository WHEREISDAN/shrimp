/** The webview runs on Windows (WebView2) rather than macOS (WebKit). */
export const IS_WINDOWS = typeof navigator !== "undefined" && /Windows/i.test(navigator.userAgent);

interface ModifierEvent {
  metaKey: boolean;
  ctrlKey: boolean;
}

/** Cmd on macOS, Ctrl on Windows: the modifier of app shortcuts such as Find. */
export function primaryKey(event: ModifierEvent, windows = IS_WINDOWS): boolean {
  return windows ? event.ctrlKey && !event.metaKey : event.metaKey;
}

/** How the primary modifier is written in shortcut hints. */
export function primaryLabel(windows = IS_WINDOWS): string {
  return windows ? "Ctrl+" : "⌘";
}
