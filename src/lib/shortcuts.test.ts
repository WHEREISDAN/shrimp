import { describe, expect, it } from "vitest";
import { keyName, shortcutLabel, shortcutText } from "./shortcuts";

const none = { ctrlKey: false, altKey: false, shiftKey: false, metaKey: false };

describe("shortcuts", () => {
  it("names physical keys the way handy-keys parses them", () => {
    expect(keyName("KeyO")).toBe("O");
    expect(keyName("Digit1")).toBe("1");
    expect(keyName("Enter")).toBe("Return");
    expect(keyName("F13")).toBe("F13");
    expect(keyName("ArrowUp")).toBe("Up");
    expect(keyName("IntlBackslash")).toBeNull();
  });

  it("writes modifiers in a fixed order", () => {
    expect(shortcutText({ ...none, altKey: true, shiftKey: true }, "Return")).toBe(
      "Opt+Shift+Return",
    );
    expect(shortcutText({ ...none, ctrlKey: true, altKey: true }, null)).toBe("Ctrl+Opt");
  });

  it("labels shortcuts with macOS symbols", () => {
    expect(shortcutLabel("Opt+Return")).toBe("⌥↩");
    expect(shortcutLabel("Ctrl+Cmd+V")).toBe("⌃⌘V");
    expect(shortcutLabel("Fn+Ctrl")).toBe("fn ⌃");
  });
});
