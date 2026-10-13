import { invoke } from "@tauri-apps/api/core";

const isTextField = (element: Element | null): boolean =>
  element instanceof HTMLTextAreaElement ||
  (element instanceof HTMLInputElement &&
    ["text", "search", "email", "url", ""].includes(element.type)) ||
  (element instanceof HTMLElement && element.isContentEditable);

/**
 * Tells Rust whether one of this window's text fields has focus, so a
 * dictation into Shrimp itself is pasted there (onboarding's practice).
 */
export function reportOwnTextFields() {
  let focused = false;
  const report = () => {
    const next = document.hasFocus() && isTextField(document.activeElement);
    if (next !== focused) {
      focused = next;
      void invoke("own_field_focused", { focused }).catch(() => undefined);
    }
  };
  window.addEventListener("focusin", report);
  window.addEventListener("focusout", () => setTimeout(report, 0));
  window.addEventListener("blur", report);
  window.addEventListener("focus", report);
}
