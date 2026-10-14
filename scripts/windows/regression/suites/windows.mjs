// Cases only Windows has: its default and reserved shortcuts (WIN-02, WIN-03),
// paste rules by executable name (WIN-04, INS-10, INS-11, STA-06), clipboard
// history (WIN-05), the tray (WIN-09), the quiet start (WIN-10), and portable
// features the macOS walkthrough leaves to a person or a test (DIC-07, FMT-14).
// Apps are imitated by running the target app under their executable names.

import { execFileSync, spawn } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { expect, expectIncludes, sleep } from "../../../macos/regression/context.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const HELLO = "Hello from Shrimp";

/** The defaults Settings lists on Windows (WIN-02). */
const DEFAULTS = {
  pushToTalk: "Ctrl+Win",
  handsFree: "Ctrl+Win+Space",
  commandMode: "Ctrl+Win+Alt",
  pasteLast: "Shift+Alt+Z",
  copyLast: "Shift+Alt+X",
  polish: "Win+Alt+1",
  promptEngineer: "Win+Alt+2",
  viewChanges: "Win+Alt+O",
};

/** Records a shortcut for `action` while `press` sends keys, and returns the final status. */
async function record(ctx, action, press) {
  await ctx.arm("shortcuts://recording", 30_000);
  const since = await ctx.capturedCursor();
  await ctx.invoke("record_shortcut", { action, index: null });
  await sleep(300);
  press();
  const statuses = await ctx.waitFor(
    "the recording result",
    async () => {
      const events = (await ctx.captured("shortcuts://recording", since)).map(
        (event) => event.payload,
      );
      return events.some((status) => status.state && status.state !== "recording") ? events : null;
    },
    8_000,
  );
  return statuses.findLast((status) => status.state !== "recording");
}

async function startTarget(ctx, exe) {
  spawn(exe, [], { detached: true, stdio: "ignore" }).unref();
  await ctx.waitFor("the target app", async () => ctx.helper("target").running, 10_000);
  await sleep(800);
  ctx.focusTarget();
}

async function quitTarget(ctx) {
  ctx.helper("focus", "quit");
  await ctx.waitFor("the target app to quit", async () => !ctx.helper("target").running, 10_000);
}

/** Runs `work` with the target app running as `exe`, then brings the usual target back. */
async function as(ctx, exe, work) {
  const original = join(ctx.workDir, "ShrimpRegressionTarget.exe");
  const copy = join(ctx.workDir, "as", exe);
  mkdirSync(dirname(copy), { recursive: true });
  copyFileSync(original, copy);
  await quitTarget(ctx);
  try {
    await startTarget(ctx, copy);
    return await work();
  } finally {
    await quitTarget(ctx).catch(() => {});
    await startTarget(ctx, original);
  }
}

/**
 * Dictates "hello" over a clipboard holding a sentinel, and measures how long
 * after the paste the sentinel came back.
 */
async function dictateAndTimeTheHold(ctx) {
  await ctx.source("hello");
  ctx.setText("");
  ctx.setClipboard("hold sentinel");
  ctx.helper("hold", "fn", 4.5);
  await ctx.waitFor("the paste", async () => ctx.text().includes("Hello"), 20_000);
  const pasted = Date.now();
  await ctx.waitFor(
    "the clipboard to come back",
    async () => ctx.clipboard() === "hold sentinel",
    10_000,
  );
  await ctx.waitIdle();
  return { text: ctx.text(), holdMs: Date.now() - pasted, key: ctx.helper("paste-key").key };
}

function history() {
  const out = execFileSync(
    "powershell",
    ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", join(here, "../history.ps1")],
    { encoding: "utf8" },
  ).trim();
  return out ? JSON.parse(out) : [];
}

export const windows = [
  {
    ids: ["WIN-02", "WIN-03"],
    name: "the Windows defaults; Start stays closed while Ctrl+Win is held; reserved shortcuts are refused",
    async run(ctx) {
      // The transforms' rows are listed while Transforms are on.
      await ctx.settings({ transforms: { enabled: true } });
      const { rows } = await ctx.invoke("get_shortcuts");
      await ctx.settings({ transforms: { enabled: false } });
      for (const [action, label] of Object.entries(DEFAULTS)) {
        const row = rows.find((item) => item.action === action);
        expect(row?.shortcuts?.[0] === label, `${action}: ${JSON.stringify(row?.shortcuts)}`);
      }
      await ctx.source("silence");
      ctx.focusTarget();
      ctx.helper("hold", "fn", 1.5);
      await ctx.waitIdle();
      await sleep(600);
      expect(ctx.frontmost() === "ShrimpRegressionTarget.exe", `in front: ${ctx.frontmost()}`);
      try {
        const ctrl = await record(ctx, "copyLast", () => ctx.helper("tap", "ctrl"));
        expectIncludes(JSON.stringify(ctrl), "This shortcut is not allowed: Ctrl");
        const copy = await record(ctx, "copyLast", () => ctx.helper("tap", "ctrl+c"));
        expectIncludes(JSON.stringify(copy), "This shortcut is not allowed: Ctrl+C");
      } finally {
        await ctx.invoke("cancel_shortcut_recording").catch(() => {});
        await ctx.settings({ shortcuts: { bindings: { copyLast: null } } });
      }
    },
  },
  {
    ids: ["WIN-04", "INS-11"],
    name: "code editors and terminals (Code, Hyper, WezTerm) paste with Shift+Insert, other apps with Ctrl+V",
    async run(ctx) {
      await ctx.settings({ ai: { cleanup: "none" } });
      const plain = await dictateAndTimeTheHold(ctx);
      expect(plain.key === "ctrl+v", `the target app: ${plain.key}`);
      for (const exe of ["Code.exe", "Hyper.exe", "wezterm-gui.exe"]) {
        const result = await as(ctx, exe, () => dictateAndTimeTheHold(ctx));
        expectIncludes(result.text, HELLO, exe);
        expect(result.key === "shift+insert", `${exe}: ${result.key}`);
      }
    },
  },
  {
    ids: ["WIN-04", "STA-06"],
    name: "Word holds the clipboard about 3 s and counts as a document",
    async run(ctx) {
      await ctx.settings({ ai: { cleanup: "none" } });
      const usage = async () =>
        (await ctx.invoke("get_insights")).usage.find((row) => row.category === "documents")
          ?.words ?? 0;
      const before = await usage();
      const word = await as(ctx, "WINWORD.EXE", () => dictateAndTimeTheHold(ctx));
      expect(word.holdMs >= 2_500 && word.holdMs <= 4_500, `held ${word.holdMs} ms`);
      expect((await usage()) > before, "the document words didn't grow");
    },
  },
  {
    ids: ["INS-10"],
    name: "a remote desktop holds the clipboard about 5 s and gets no context spacing",
    async run(ctx) {
      await ctx.settings({ ai: { cleanup: "none" } });
      await as(ctx, "mstsc.exe", async () => {
        const remote = await dictateAndTimeTheHold(ctx);
        expect(remote.holdMs >= 4_500 && remote.holdMs <= 6_500, `held ${remote.holdMs} ms`);
        ctx.setText("We should talk about it");
        const text = await ctx.dictate("tomorrow", { hold: 3, keepText: true });
        expect(/about itTomorrow at noon/.test(text), `inserted: ${JSON.stringify(text)}`);
      });
    },
  },
  {
    ids: ["INS-08", "FMT-06"],
    name: "spoken line breaks show as lines in a classic edit control",
    async run(ctx) {
      await ctx.settings({ ai: { cleanup: "none" } });
      const text = await ctx.dictate("lines", { hold: 7 });
      const { lines } = ctx.helper("field");
      expect(lines >= 4, `${lines} lines shown for ${JSON.stringify(text)}`);
    },
  },
  {
    ids: ["WIN-05", "INS-06"],
    name: "Shrimp's pastes stay out of clipboard history; Copy last transcript goes in",
    async run(ctx) {
      await ctx.settings({ ai: { cleanup: "none" } });
      const before = new Set(history().map((item) => item.id));
      const text = await ctx.dictate("hello", { hold: 4.5 });
      expectIncludes(text, HELLO);
      await sleep(1500);
      const pasted = history().filter((item) => !before.has(item.id));
      expect(
        !pasted.some((item) => item.text.includes(HELLO)),
        `in history: ${JSON.stringify(pasted)}`,
      );
      ctx.helper("tap", "ctrl+cmd+c");
      await ctx.waitFor(
        "the copy in clipboard history",
        async () => history().some((item) => !before.has(item.id) && item.text.includes(HELLO)),
        5_000,
      );
    },
  },
  {
    ids: ["STA-03"],
    name: "the share card copies as an image and downloads as a PNG",
    async run(ctx) {
      const click = (label) =>
        ctx.app.send("execute_js", {
          code: `(() => { const button = [...document.querySelectorAll("button")].find((element) => element.textContent.trim() === ${JSON.stringify(label)}); button?.click(); return Boolean(button); })()`,
          window_label: "main",
        });
      await ctx.app.send("manage_ipc", {
        action: "emit",
        event: "app://navigate",
        payload: "insights",
        window_label: "main",
      });
      await sleep(800);
      // Cards this case saves into Downloads, deleted whatever happens.
      const downloads = join(homedir(), "Downloads");
      const cards = () =>
        new Set(readdirSync(downloads).filter((name) => name.startsWith("My Shrimp activity")));
      const before = cards();
      try {
        await click("Share");
        await sleep(1000);
        ctx.setClipboard("share sentinel");
        await click("Copy image");
        await ctx.waitFor(
          "the image on the clipboard",
          async () => ctx.helper("clipboard-formats").includes("PNG"),
          5_000,
        );
        await click("Download");
        let status = "";
        const saved = await ctx
          .waitFor(
            "the saved card",
            async () => {
              status = (await ctx.app.readText("[role=dialog] [role=status]")).join(" ");
              return status.match(/Saved to (.+?\.png)/)?.[1];
            },
            8_000,
          )
          .catch((error) => {
            throw new Error(`${error.message}: ${JSON.stringify(status)}`, { cause: error });
          });
        expect(
          existsSync(saved) && readFileSync(saved).subarray(1, 4).toString() === "PNG",
          `saved ${saved}`,
        );
      } finally {
        for (const name of cards()) if (!before.has(name)) rmSync(join(downloads, name));
        await click("Close");
        ctx.focusTarget();
      }
    },
  },
  {
    ids: ["WIN-09"],
    name: "after a start the tray's Paste last transcript is greyed until there is a transcript",
    async run(ctx) {
      // Shrimp logs the item's state whenever it rebuilds the menu; opening the
      // menu needs the icon outside the taskbar's overflow flyout (APP-03).
      const states = async () =>
        (await ctx.logs(0))
          .map((line) => /tray menu: paste last transcript enabled: (\w+)/.exec(line)?.[1])
          .filter(Boolean);
      await ctx.restart();
      expect((await states())[0] === "false", `after the start: ${await states()}`);
      await ctx.dictate("hello", { hold: 4.5 });
      await ctx.waitFor("the enabled item", async () => (await states()).at(-1) === "true", 5_000);
    },
  },
  {
    ids: ["DIC-07", "WIN-11"],
    name: "a word respelled after a paste is added to the dictionary",
    async run(ctx) {
      // A word of its own: auto-learn never adds back a word deleted before.
      const word = "Ngwenya";
      await ctx.settings({ ai: { cleanup: "none" }, dictionary: { autoLearn: true } });
      const cleanUp = async () => {
        const entries = await ctx.invoke("list_dictionary");
        const ids = entries.filter((entry) => entry.term === word).map((entry) => entry.id);
        if (ids.length) await ctx.invoke("delete_dictionary_entries", { ids });
      };
      try {
        const text = await ctx.dictate("nguyen", { hold: 4.5 });
        const heard = /^Ask (\w+)/.exec(text)?.[1];
        expect(heard && heard !== word, `inserted ${JSON.stringify(text)}`);
        const since = await ctx.cursor();
        ctx.setText(text.replace(heard, word));
        await ctx.waitNotice(since, new RegExp(`Added “${word}” to your dictionary`), 10_000);
        const entry = (await ctx.invoke("list_dictionary")).find((item) => item.term === word);
        expect(entry?.source === "learned", `entry: ${JSON.stringify(entry)}`);
      } finally {
        await cleanUp();
      }
    },
  },
  {
    ids: ["FMT-14"],
    name: "French gets a narrow no-break space before ? : ; !",
    async run(ctx) {
      try {
        await ctx.invoke("set_languages", { autoDetect: false, selected: ["en-US", "fr"] });
        await ctx.settings({ ai: { cleanup: "none" } });
        const text = await ctx.dictate("french", { hold: 6 });
        expectIncludes(text, "Bonjour");
        expect(/\u202f[?:;!]/.test(text), `no narrow no-break space: ${JSON.stringify(text)}`);
      } finally {
        await ctx.invoke("set_languages", { autoDetect: false, selected: ["en-US"] });
      }
    },
  },
  {
    ids: ["WIN-10"],
    name: "started with --background (the login item's command), the main window stays hidden",
    async run(ctx) {
      try {
        await ctx.restart({}, ["--background"]);
        expect(!(await ctx.windowVisible("main")), "the main window opened");
        expect(await ctx.windowVisible("flow-bar"), "no Flow Bar");
      } finally {
        await ctx.restart();
        await ctx.source("hello");
        ctx.focusTarget();
      }
    },
  },
];
