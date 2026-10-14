// What every walkthrough case uses: the app over tauri-mcp, the test helper
// (keys, clipboard, Accessibility), the target app and the WAV fixtures.

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

export const DATA_DIR = join(homedir(), "Library/Application Support/com.shrimp.desktop.dev");

export const TARGET = "com.shrimp.regression.target";
export const TARGET_NAME = "Shrimp Regression Target";

export class Skip extends Error {}

/** The pipeline worked but a model's output missed a quality check. */
export class Soft extends Error {}

export const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

export class Ctx {
  constructor({ app, helper, fixtures, workDir, ollama, bundle }) {
    this.app = app;
    this.bundle = bundle;
    this.helperPath = helper;
    this.fixtures = fixtures;
    this.workDir = workDir;
    this.ollama = ollama;
    this.notes = [];
  }

  // MARK: helper

  helper(...args) {
    const out = execFileSync(this.helperPath, args.map(String), { encoding: "utf8" }).trim();
    return out ? JSON.parse(out) : null;
  }

  frontmost() {
    return this.helper("frontmost").bundleId;
  }

  clipboard() {
    return this.helper("clipboard-text");
  }

  setClipboard(text) {
    this.helper("clipboard-set", text);
  }

  // MARK: target app

  /** Brings the target to the front through LaunchServices, which macOS allows a background process. */
  focusTarget(field = "text") {
    if (this.frontmost() !== TARGET) {
      execFileSync("open", [join(this.workDir, `${TARGET_NAME}.app`)]);
      execFileSync("sleep", ["0.4"]);
    }
    this.helper("notify", TARGET, field);
  }

  /** The focused field's text in the target app. */
  text() {
    return this.helper("field", TARGET).value;
  }

  /** Replaces the focused field's text, with the cursor at `cursor` (default: the end). */
  setText(text, cursor) {
    this.focusTarget();
    this.helper("set-field", text, TARGET);
    this.helper("select", cursor ?? [...text].length, 0, TARGET);
  }

  select(start, length) {
    this.helper("select", start, length, TARGET);
  }

  // MARK: app

  invoke(command, args) {
    return this.app.invoke(command, args);
  }

  settings(patch) {
    return this.app.invoke("update_settings", { patch });
  }

  async state() {
    return (await this.app.invoke("get_dictation_state")).phase;
  }

  async waitFor(what, condition, timeoutMs = 15_000) {
    const deadline = Date.now() + timeoutMs;
    for (;;) {
      const value = await condition();
      if (value) return value;
      if (Date.now() > deadline) throw new Error(`timed out waiting for ${what}`);
      await sleep(100);
    }
  }

  async waitIdle(timeoutMs = 20_000) {
    await this.waitFor("idle", async () => (await this.state()) === "idle", timeoutMs);
  }

  /** The id of the newest log entry, to read only what comes after it. */
  async cursor() {
    const data = await this.app.send("query_logs", { mode: "tail", limit: 1 });
    return data?.nextCursor ?? 0;
  }

  async logs(since) {
    const data = await this.app.logs(since);
    return (data?.entries ?? []).map((entry) => entry.message);
  }

  /** The notices shown since `since`, as their text. */
  async notices(since) {
    return (await this.logs(since))
      .map((line) => /^notice \(\w+\): (.*)$/s.exec(line)?.[1])
      .filter(Boolean);
  }

  async waitNotice(since, pattern, timeoutMs = 15_000) {
    return this.waitFor(
      `a notice matching ${pattern}`,
      async () => (await this.notices(since)).find((notice) => pattern.test(notice)),
      timeoutMs,
    );
  }

  async waitLog(since, pattern, timeoutMs = 15_000) {
    return this.waitFor(
      `a log line matching ${pattern}`,
      async () => (await this.logs(since)).find((line) => pattern.test(line)),
      timeoutMs,
    );
  }

  fixture(name) {
    const path = join(this.fixtures, `${name}.wav`);
    if (!existsSync(path)) throw new Error(`missing fixture ${path}`);
    return path;
  }

  async source(name) {
    await this.app.invoke("dev_set_audio_source", { spec: `wav:${this.fixture(name)}` });
  }

  /**
   * Holds fn over a WAV fixture with the target app's text view focused and
   * empty, and returns what was inserted once the dictation is over.
   */
  async dictate(name, { hold = 6, keepText = false, field = "text", button = "fn" } = {}) {
    await this.source(name);
    if (!keepText) this.setText("");
    this.focusTarget(field);
    const before = this.text();
    this.helper("hold", button, hold);
    await this.waitIdle();
    await this.waitFor("the paste", async () => this.text() !== before, 4_000).catch(() => {});
    await sleep(700);
    return this.text();
  }

  /** The `aria-label`s of the elements matching `selector` in a webview. */
  async labels(selector, window = "main") {
    const data = await this.app.send("read_text", {
      selector,
      window_label: window,
      all: true,
      limit: 50,
      attrs: ["aria-label"],
    });
    return (data?.matches ?? []).map((match) => match.attrs?.["aria-label"] ?? match.text);
  }

  async windowVisible(label) {
    const info = await this.app.send("get_app_info", {});
    return Boolean(info?.windows?.find((window) => window.label === label)?.visible);
  }

  readSettingsFile() {
    return JSON.parse(readFileSync(join(DATA_DIR, "settings.json"), "utf8"));
  }

  appPid() {
    return Number(
      execFileSync("pgrep", ["-f", "Shrimp Dev.app/Contents/MacOS/shrimp"], { encoding: "utf8" })
        .trim()
        .split("\n")[0],
    );
  }

  /** Records every `event` from now on into the plugin's IPC buffer. */
  async arm(event, timeoutMs = 300_000) {
    await this.app.send("manage_ipc", {
      action: "arm_event",
      event,
      timeout_ms: timeoutMs,
      window_label: "main",
    });
  }

  /** The newest entry in the plugin's IPC buffer, to read only later captures. */
  async capturedCursor() {
    const data = await this.app.send("manage_ipc", {
      action: "captured",
      limit: 1,
      window_label: "main",
    });
    return Math.max(0, ...(data?.entries ?? []).map((entry) => entry.id));
  }

  /** Payloads of `event` captured after the IPC buffer entry `sinceId`. */
  async captured(event, sinceId = 0) {
    const data = await this.app.send("manage_ipc", {
      action: "captured",
      kind: "event",
      name_contains: event,
      since_id: sinceId,
      limit: 200,
      window_label: "main",
    });
    return (data?.entries ?? []).map((entry) => ({
      id: entry.id,
      payload: parse(entry.result_preview),
    }));
  }

  /** After a failed case: no dictation running, no key left down, raw transcripts. */
  async recover() {
    for (const key of ["fn", "ctrl", "opt", "cmd", "shift"]) this.helper("up", key);
    if ((await this.state()) !== "idle") {
      await this.invoke("flow_bar_control", { control: "cancel" });
      await this.waitIdle();
    }
    await this.settings({ ai: { cleanup: "none" } });
  }

  requireOllama() {
    if (!this.ollama) throw new Skip("Ollama isn't running");
  }

  note(text) {
    this.notes.push(text);
  }
}

function parse(text) {
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

export function expect(condition, message) {
  if (!condition) throw new Error(message);
}

export function soft(condition, message) {
  if (!condition) throw new Soft(message);
}

export function expectIncludes(text, part, what = "text") {
  if (
    !String(text ?? "")
      .toLowerCase()
      .includes(part.toLowerCase())
  ) {
    throw new Error(`${what} ${JSON.stringify(text)} lacks ${JSON.stringify(part)}`);
  }
}
