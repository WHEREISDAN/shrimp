import { mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";
import { handleContextMenu, inTitleArea } from "./windowChrome";

function pathOf(element: Element): EventTarget[] {
  const path: EventTarget[] = [];
  for (let node: Element | null = element; node; node = node.parentElement) {
    path.push(node);
  }
  return [...path, document, window];
}

function rightClick(target: Element) {
  const event = new MouseEvent("contextmenu", { bubbles: true, cancelable: true });
  Object.defineProperty(event, "target", { value: target });
  event.composedPath = () => pathOf(target);
  handleContextMenu(event);
  return event;
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("the title area", () => {
  it("follows Tauri's drag regions", () => {
    document.body.innerHTML = `
      <div data-tauri-drag-region="deep" id="toolbar"><span id="title">Settings</span><button id="button">Go</button></div>
      <div data-tauri-drag-region id="band"><span id="inside">x</span></div>
      <div data-tauri-drag-region="deep"><div data-tauri-drag-region="false"><span id="off">x</span></div></div>
      <p id="content">Text</p>`;
    const at = (id: string) => pathOf(document.getElementById(id) as Element);
    expect(inTitleArea(at("toolbar"))).toBe(true);
    expect(inTitleArea(at("title"))).toBe(true);
    expect(inTitleArea(at("button"))).toBe(false);
    expect(inTitleArea(at("band"))).toBe(true);
    expect(inTitleArea(at("inside"))).toBe(false);
    expect(inTitleArea(at("off"))).toBe(false);
    expect(inTitleArea(at("content"))).toBe(false);
  });

  it("opens the window menu there and keeps the browser menu for text fields only", () => {
    const calls: string[] = [];
    mockIPC((cmd) => {
      calls.push(cmd);
      return null;
    });
    document.body.innerHTML = `
      <div data-tauri-drag-region="deep"><h1 id="title">Home</h1></div>
      <p id="content">Text</p>
      <input id="field" />`;
    const element = (id: string) => document.getElementById(id) as Element;

    expect(rightClick(element("title")).defaultPrevented).toBe(true);
    expect(calls).toEqual(["show_window_menu"]);
    expect(rightClick(element("content")).defaultPrevented).toBe(true);
    expect(rightClick(element("field")).defaultPrevented).toBe(false);
    expect(calls).toEqual(["show_window_menu"]);
  });
});
