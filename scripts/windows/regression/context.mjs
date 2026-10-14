// The Windows side of the regression walkthrough. The cases are the macOS
// ones (scripts/macos/regression/suites); this context runs them against
// Shrimp Dev on Windows. It turns the macOS default shortcuts the cases press
// into the Windows defaults (WIN-02), reaches the target app, the clipboard
// and the tray through shrimp-helper.exe (helper.cs), and stops the run when
// the guard says so.

import { execFileSync, spawn } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";

import { Ctx, Skip, sleep } from "../../macos/regression/context.mjs";
import { McpClient } from "../../macos/regression/mcp-client.mjs";

/** The macOS defaults the cases press, as the Windows defaults (WIN-02). */
const KEYS = {
  fn: "ctrl+win",
  "fn+space": "ctrl+win+space",
  // Alt first, so the press goes straight to Command Mode instead of through a dictation.
  "ctrl+fn": "alt+ctrl+win",
  "ctrl+cmd+v": "shift+alt+z",
  "ctrl+cmd+c": "shift+alt+x",
  "opt+1": "win+alt+1",
  "opt+2": "win+alt+2",
  "opt+o": "win+alt+o",
  "opt+return": "alt+return",
  "ctrl+opt+k": "ctrl+alt+k",
  "cmd+z": "ctrl+z",
  "cmd+v": "ctrl+v",
  delete: "backspace",
};
const KEY_COMMANDS = new Set(["down", "up", "tap", "hold", "measure"]);
const MAC_ONLY = /\b(fn|cmd|opt)\b/;

/** The run stopped: the person at the PC used it, or another app came to the front. */
export class Aborted extends Error {}

function windowsKeys(spec) {
  const keys = KEYS[spec] ?? spec;
  if (MAC_ONLY.test(keys)) throw new Error(`no Windows equivalent for ${spec}`);
  return keys;
}

export class WinCtx extends Ctx {
  pushToTalkLabel = "Ctrl+Win";
  pushToTalkBinding = "Ctrl+Cmd";
  // Ctrl+Win+F is the shell's Find Computers; Ctrl+Win+J does nothing.
  interruptKey = "j";
  trayClickOpensWindow = true;

  constructor({ exe, devApp, abortFile, ...options }) {
    super(options);
    this.exe = exe;
    this.devApp = devApp;
    this.abortFile = abortFile;
    /** Combinations pressed with `down` and not yet released. */
    this.held = [];
  }

  /** The guard's reason to stop, or null. */
  guardReason() {
    return existsSync(this.abortFile) ? readFileSync(this.abortFile, "utf8").trim() : null;
  }

  /**
   * Why the run has to stop after a case, or null: the guard's reason, or a
   * window of another app in front, which no key may reach.
   */
  stopped() {
    const reason = this.guardReason();
    if (reason) return reason;
    const front = this.helper("frontmost");
    if (front.title === "Shrimp Regression Target" || front.exe === "shrimp.exe") return null;
    const foreign = `${front.exe || "no app"} ("${front.title}") came to the front`;
    writeFileSync(this.abortFile, foreign);
    return foreign;
  }

  helper(command, ...args) {
    if (KEY_COMMANDS.has(command)) args[0] = windowsKeys(args[0]);
    const reason = command === "up" ? null : this.guardReason();
    if (reason) {
      this.releaseHeld();
      throw new Aborted(reason);
    }
    let out;
    try {
      out = execFileSync(this.helperPath, [command, ...args.map(String)], {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "pipe"],
      }).trim();
    } catch (error) {
      const message = String(error.stderr ?? error.message).trim();
      if (error.status === 3) {
        this.releaseHeld();
        throw new Aborted(message, { cause: error });
      }
      // The tray icon is hidden in the taskbar's overflow flyout, which belongs to Explorer.
      if (error.status === 4) throw new Skip(message, { cause: error });
      throw new Error(`helper ${command}: ${message}`, { cause: error });
    }
    if (command === "down") this.held.push(args[0]);
    if (command === "up") this.held = this.held.filter((combo) => combo !== args[0]);
    return out ? JSON.parse(out) : null;
  }

  releaseHeld() {
    for (const combo of this.held.reverse()) {
      try {
        execFileSync(this.helperPath, ["up", combo], { stdio: "ignore" });
      } catch {
        // Releasing what is left is all that matters here.
      }
    }
    this.held = [];
  }

  frontmost() {
    return this.helper("frontmost").exe;
  }

  clipboard() {
    return this.helper("clipboard-text");
  }

  setClipboard(text) {
    this.helper("clipboard-set", text);
  }

  /** Brings the target to the front and focuses one of its controls. */
  focusTarget(field = "text") {
    this.helper("focus", field);
  }

  text() {
    return this.helper("field").value;
  }

  setText(text, cursor) {
    this.focusTarget();
    this.helper("set-field", text);
    this.select(cursor ?? [...text].length, 0);
  }

  select(start, length) {
    this.helper("select", start, length);
  }

  /** Shrimp Dev's process, or null while it isn't running. */
  runningPid() {
    const out = execFileSync("tasklist", ["/FI", "IMAGENAME eq shrimp.exe", "/FO", "CSV", "/NH"], {
      encoding: "utf8",
    });
    const pid = /"shrimp\.exe","(\d+)"/i.exec(out)?.[1];
    return pid ? Number(pid) : null;
  }

  appPid() {
    const pid = this.runningPid();
    if (!pid) throw new Error("Shrimp Dev isn't running");
    return pid;
  }

  /** The process's TCP and UDP endpoints (PRIV-01), or "" when there are none. */
  sockets(pid) {
    return execFileSync("netstat", ["-ano"], { encoding: "utf8" })
      .split(/\r?\n/)
      .filter(
        (line) => /^\s*(TCP|UDP)\s/.test(line) && line.trim().split(/\s+/).at(-1) === `${pid}`,
      )
      .join("\n");
  }

  launchAgain() {
    spawn(this.exe, [], { detached: true, stdio: "ignore" }).unref();
  }

  trayItems(item) {
    return this.helper("tray", ...(item ? [item] : []));
  }

  /** What Explorer sends Shrimp for a click on its notification-area icon. */
  trayClick(button) {
    this.helper("tray-click", button);
  }

  killApp() {
    execFileSync("taskkill", ["/F", "/PID", String(this.appPid())], { stdio: "ignore" });
  }

  connect() {
    return McpClient.connect();
  }

  async relaunched(action, timeoutMs = 60_000) {
    const before = this.runningPid();
    this.app.close();
    try {
      await action();
    } catch (error) {
      this.app = await this.connect();
      throw error;
    }
    await this.waitFor(
      "the restarted app",
      async () => {
        const pid = this.runningPid();
        return pid && pid !== before;
      },
      timeoutMs,
    );
    this.app = await this.waitFor(
      "its tauri-mcp pipe",
      async () => this.connect().catch(() => null),
      30_000,
    );
    await sleep(1500);
  }

  /** Waits until Whisper is loaded: a dictation before that only says the model is loading. */
  async waitForModel() {
    await this.waitFor(
      "the speech model",
      async () =>
        (await this.invoke("get_models").catch(() => [])).some(
          (model) => model.inUse && model.state === "active",
        ),
      120_000,
    );
  }

  /** Quits Shrimp Dev and opens it again with `env` and `args` (dev-app.ps1 open). */
  async restart(env = {}, args = []) {
    const variables = { SHRIMP_AUDIO_SOURCE: `wav:${this.fixture("silence")}`, ...env };
    await this.relaunched(async () => {
      execFileSync(
        "powershell",
        [
          "-NoProfile",
          "-ExecutionPolicy",
          "Bypass",
          "-File",
          this.devApp,
          "open",
          ...args,
          ...Object.entries(variables).map(([name, value]) => `${name}=${value}`),
        ],
        { stdio: "ignore" },
      );
    }, 30_000);
    await this.waitFor(
      "the app to answer",
      async () => (await this.invoke("get_dictation_state").catch(() => null)) !== null,
      30_000,
    );
    await this.waitForModel();
    await sleep(1500);
  }

  async recover() {
    this.releaseHeld();
    if (this.stopped()) return;
    await super.recover();
  }
}
