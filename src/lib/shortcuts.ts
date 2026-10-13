/** Turns browser key events into handy-keys shortcuts ("Opt+Return") and shortcuts into labels ("⌥↩"). */

const NAMED_KEYS: Record<string, string> = {
  Enter: "Return",
  NumpadEnter: "KeypadEnter",
  Space: "Space",
  Tab: "Tab",
  Backspace: "Backspace",
  Escape: "Escape",
  Delete: "ForwardDelete",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  ArrowUp: "Up",
  ArrowDown: "Down",
  Minus: "Minus",
  Equal: "Equal",
  BracketLeft: "LeftBracket",
  BracketRight: "RightBracket",
  Backslash: "Backslash",
  Semicolon: "Semicolon",
  Quote: "Quote",
  Comma: "Comma",
  Period: "Period",
  Slash: "Slash",
  Backquote: "Grave",
};

const MODIFIER_CODES = new Set([
  "ShiftLeft",
  "ShiftRight",
  "ControlLeft",
  "ControlRight",
  "AltLeft",
  "AltRight",
  "MetaLeft",
  "MetaRight",
  "CapsLock",
  "Fn",
]);

const KEY_LABELS: Record<string, string> = {
  Return: "↩",
  Backspace: "⌫",
  ForwardDelete: "⌦",
  Escape: "Esc",
  Tab: "⇥",
  Left: "←",
  Right: "→",
  Up: "↑",
  Down: "↓",
};

const MODIFIER_LABELS: [string, string][] = [
  ["Ctrl", "⌃"],
  ["Opt", "⌥"],
  ["Shift", "⇧"],
  ["Cmd", "⌘"],
];

export interface ModifierState {
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}

export function isModifierCode(code: string): boolean {
  return MODIFIER_CODES.has(code);
}

/** The handy-keys name of a physical key (`KeyboardEvent.code`), or null if it can't be bound. */
export function keyName(code: string): string | null {
  const letter = /^Key([A-Z])$/.exec(code);
  if (letter?.[1]) {
    return letter[1];
  }
  const digit = /^Digit(\d)$/.exec(code);
  if (digit?.[1]) {
    return digit[1];
  }
  const keypad = /^Numpad(\d)$/.exec(code);
  if (keypad?.[1]) {
    return `Keypad${keypad[1]}`;
  }
  if (/^F([1-9]|1\d|2[0-4])$/.test(code)) {
    return code;
  }
  return NAMED_KEYS[code] ?? null;
}

/** "Ctrl+Opt+Shift+Cmd+Key" for the held modifiers and an optional key. */
export function shortcutText(modifiers: ModifierState, key: string | null): string {
  const parts = [
    modifiers.ctrlKey && "Ctrl",
    modifiers.altKey && "Opt",
    modifiers.shiftKey && "Shift",
    modifiers.metaKey && "Cmd",
    key,
  ].filter((part): part is string => Boolean(part));
  return parts.join("+");
}

/** A shortcut in macOS notation: "Opt+Return" becomes "⌥↩". */
export function shortcutLabel(text: string): string {
  const parts = text.split("+");
  const fn = parts.some((part) => part.toLowerCase() === "fn");
  const modifiers = MODIFIER_LABELS.filter(([name]) => parts.includes(name))
    .map(([, symbol]) => symbol)
    .join("");
  const key = parts.find(
    (part) => !MODIFIER_LABELS.some(([name]) => name === part) && part.toLowerCase() !== "fn",
  );
  const keyLabel = key ? (KEY_LABELS[key] ?? key) : "";
  return `${fn ? "fn " : ""}${modifiers}${keyLabel}`;
}
