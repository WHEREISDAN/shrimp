// Shortcut customisation, microphones, history, insights, languages,
// notifications, privacy and the app shell (phase 4).

import { existsSync, readFileSync, readdirSync, rmSync } from "node:fs";
import { join } from "node:path";

import { DATA_DIR, expect, expectIncludes, sleep, soft } from "../context.mjs";
import { withOllama } from "./editing.mjs";

const WAV = (ctx, name) => ({ type: "wav", path: ctx.fixture(name) });

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

async function fakeMicrophones(ctx, devices, extra = {}) {
  await ctx.invoke("dev_set_audio_source", { spec: "mic" });
  return ctx.invoke("dev_set_microphones", {
    devices,
    default: devices?.[0]?.id ?? null,
    lidClosed: false,
    ...extra,
  });
}

async function realMicrophones(ctx) {
  await ctx.invoke("dev_set_microphones", { devices: null, default: null, lidClosed: false });
  await ctx.invoke("select_microphone", { id: null });
  await ctx.settings({ microphone: { selected: null, ranking: [] } });
  await ctx.source("hello");
}

async function languages(ctx, selected, autoDetect = false) {
  return ctx.invoke("set_languages", { autoDetect, selected });
}

export const app = [
  {
    ids: ["KEYS-01", "KEYS-03", "KEYS-05", "KEYS-06"],
    name: "recording shortcuts: saved, refused and cancelled",
    async run(ctx) {
      try {
        const refused = await record(ctx, "copyLast", () => ctx.helper("tap", "cmd+v"));
        expectIncludes(JSON.stringify(refused), "This shortcut is not allowed");
        const saved = await record(ctx, "copyLast", () => ctx.helper("tap", "ctrl+opt+k"));
        expect(saved.state === "saved", `status: ${JSON.stringify(saved)}`);
        const row = (await ctx.invoke("get_shortcuts")).rows.find(
          (item) => item.action === "copyLast",
        );
        expect(row.shortcuts.length === 2, `copy last: ${JSON.stringify(row.shortcuts)}`);
        const cancelled = await record(ctx, "copyLast", () => ctx.helper("tap", "esc"));
        expect(cancelled.state === "cancelled", `status: ${JSON.stringify(cancelled)}`);
      } finally {
        await ctx.invoke("cancel_shortcut_recording").catch(() => {});
        await ctx.settings({ shortcuts: { bindings: { copyLast: null } } });
      }
    },
  },
  {
    ids: ["KEYS-02", "KEYS-07", "KEYS-08"],
    name: "binding limits, the overlap warning and reset",
    async run(ctx) {
      const { rows } = await ctx.invoke("get_shortcuts");
      const max = Object.fromEntries(rows.map((row) => [row.action, row.max]));
      expect(max.pushToTalk === 4 && max.cancel === 1, `limits: ${JSON.stringify(max)}`);
      await ctx.settings({
        shortcuts: {
          bindings: {
            pushToTalk: ["Ctrl+Opt+Space"],
            handsFree: ["Ctrl+Opt"],
            pressEnter: ["Opt+Return"],
          },
        },
      });
      const { warning } = await ctx.invoke("get_shortcuts");
      expect(warning, "no overlap warning");
      await ctx.invoke("reset_shortcuts");
      const bindings = (await ctx.invoke("get_settings")).shortcuts.bindings;
      expect(Object.keys(bindings).length === 0, `after reset: ${JSON.stringify(bindings)}`);
    },
  },
  {
    ids: ["ENG-02", "ENG-08"],
    name: "the model list shows the five Whisper models with the default in use",
    async run(ctx) {
      const models = await ctx.invoke("get_models");
      expect(models.length === 5, `models: ${models.map((model) => model.id)}`);
      expect(
        models.every((model) => model.sizeBytes > 0),
        "a model without a size",
      );
      const active = models.find((model) => model.inUse);
      expect(active?.isDefault && active.state === "active", `in use: ${JSON.stringify(active)}`);
    },
  },
  {
    ids: ["MIC-02", "MIC-05", "MIC-06"],
    name: "the microphone list: named devices, a closed lid, virtual devices",
    async run(ctx) {
      try {
        const list = await fakeMicrophones(ctx, [
          { id: "usb", name: "Yeti USB", kind: "wired", behaviour: WAV(ctx, "hello") },
          {
            id: "builtin",
            name: "MacBook Pro Microphone",
            kind: "builtIn",
            behaviour: { type: "tone", amplitude: 0.2 },
          },
          {
            id: "blackhole",
            name: "BlackHole 2ch",
            kind: "virtual",
            behaviour: { type: "silent" },
          },
        ]);
        const names = list.devices.map((device) => device.name);
        expect(
          names.includes("Yeti USB") && names.includes("MacBook Pro Microphone"),
          `devices: ${names}`,
        );
        expect(list.active !== "blackhole", "Auto-detect chose the virtual device");
        const closed = await ctx.invoke("dev_set_microphones", {
          devices: [
            { id: "usb", name: "Yeti USB", kind: "wired", behaviour: WAV(ctx, "hello") },
            {
              id: "builtin",
              name: "MacBook Pro Microphone",
              kind: "builtIn",
              behaviour: { type: "tone", amplitude: 0.2 },
            },
          ],
          default: "builtin",
          lidClosed: true,
        });
        const builtIn = closed.devices.find((device) => device.id === "builtin");
        expectIncludes(builtIn?.unavailable, "lid is closed", "built-in microphone");
      } finally {
        await realMicrophones(ctx);
      }
    },
  },
  {
    ids: ["MIC-03", "MIC-04"],
    name: "a microphone unplugged mid-dictation falls back; a new one asks to switch",
    async run(ctx) {
      const yeti = { id: "usb", name: "Yeti USB", kind: "wired", behaviour: WAV(ctx, "padded") };
      const builtIn = {
        id: "builtin",
        name: "MacBook Pro Microphone",
        kind: "builtIn",
        behaviour: WAV(ctx, "hello"),
      };
      try {
        await fakeMicrophones(ctx, [yeti, builtIn]);
        await ctx.invoke("rank_microphone", { id: "usb", up: true }).catch(() => {});
        ctx.setText("");
        ctx.focusTarget();
        const since = await ctx.cursor();
        await ctx.invoke("flow_bar_control", { control: "start" });
        await sleep(2500);
        await ctx.invoke("dev_set_microphones", {
          devices: [builtIn],
          default: "builtin",
          lidClosed: false,
        });
        await ctx.waitNotice(since, /Yeti USB disconnected\. Switched to MacBook Pro Microphone/);
        await sleep(1500);
        await ctx.invoke("flow_bar_control", { control: "stop" });
        await ctx.waitIdle();
        await ctx.waitFor("the paste", async () => ctx.text().length > 0, 5_000);
        const plugged = await ctx.cursor();
        await ctx.invoke("dev_set_microphones", {
          devices: [
            builtIn,
            { id: "mv7", name: "Shure MV7", kind: "wired", behaviour: { type: "silent" } },
          ],
          default: "builtin",
          lidClosed: false,
        });
        await ctx.waitNotice(plugged, /Shure MV7 is connected\. Switch to it\?/);
      } finally {
        await realMicrophones(ctx);
      }
    },
  },
  {
    ids: ["MIC-07", "MIC-08"],
    name: "a silent microphone and a Bluetooth microphone each get their notice",
    async run(ctx) {
      try {
        await fakeMicrophones(ctx, [
          { id: "mute", name: "Muted mic", kind: "builtIn", behaviour: { type: "silent" } },
        ]);
        ctx.setText("");
        ctx.focusTarget();
        let since = await ctx.cursor();
        ctx.helper("hold", "fn", 4);
        await ctx.waitNotice(since, /Is your microphone muted\?/);
        await ctx.waitIdle();
        await ctx.settings({
          notices: { bluetoothAdviceAt: 0, bluetoothAdviceOff: false },
        });
        await fakeMicrophones(ctx, [
          { id: "pods", name: "AirPods Pro", kind: "bluetooth", behaviour: WAV(ctx, "hello") },
        ]);
        since = await ctx.cursor();
        ctx.helper("hold", "fn", 4);
        await ctx.waitIdle();
        await ctx.waitNotice(since, /Bluetooth microphones like AirPods Pro/);
      } finally {
        await realMicrophones(ctx);
      }
    },
  },
  {
    ids: ["MIC-09"],
    name: "whispered speech is transcribed",
    async run(ctx) {
      await ctx.settings({ ai: { cleanup: "none" } });
      expectIncludes(await ctx.dictate("whispered", { hold: 8 }), "design review");
    },
  },
  {
    ids: ["HIS-01", "HIS-02", "HIS-03", "HIS-08"],
    name: "History lists, searches and extracts dictations; the database is backed up",
    async run(ctx) {
      await ctx.settings({ ai: { cleanup: "none" } });
      await ctx.dictate("hello", { hold: 4.5 });
      const { entries } = await ctx.invoke("list_history", { query: "" });
      const latest = entries[0];
      expect(
        latest?.finalText?.includes("Hello from Shrimp") && latest.language === "en",
        `latest: ${JSON.stringify(latest)}`,
      );
      const found = await ctx.invoke("list_history", { query: "local dictation" });
      expect(
        found.entries.length > 0 &&
          found.entries.every((entry) => /local dictation/i.test(entry.finalText)),
        "search",
      );
      const path = await ctx.invoke("extract_history_audio", { id: latest.id });
      try {
        expect(
          existsSync(path) && readFileSync(path).subarray(0, 4).toString() === "RIFF",
          `extracted ${path}`,
        );
      } finally {
        if (path && existsSync(path)) rmSync(path);
      }
      const today = new Date().toLocaleDateString("en-CA");
      const backups = readdirSync(join(DATA_DIR, "backups"));
      expect(backups.includes(`shrimp-${today}.db`), `backups: ${backups}`);
    },
  },
  {
    ids: ["HIS-05"],
    name: "an empty entry can be transcribed again",
    async run(ctx) {
      await ctx.dictate("silence6", { hold: 6.5 });
      const [entry] = (await ctx.invoke("list_history", { query: "" })).entries;
      expect(entry && !entry.finalText && entry.retry, `entry: ${JSON.stringify(entry)}`);
      const since = await ctx.cursor();
      await ctx.invoke("retry_history", { id: entry.id });
      await ctx.waitNotice(since, /No speech detected/);
    },
  },
  {
    ids: ["FMT-12"],
    name: "Undo AI edit shows the raw transcript and Redo brings the edit back",
    async run(ctx) {
      await withOllama(ctx);
      await ctx.dictate("filler", { hold: 5 });
      await ctx.waitIdle();
      const [entry] = (await ctx.invoke("list_history", { query: "" })).entries;
      soft(
        entry.formatted && entry.rawText !== entry.finalText,
        `nothing to undo: ${JSON.stringify(entry)}`,
      );
      const raw = await ctx.invoke("set_history_raw", { id: entry.id, raw: true });
      expect(raw.showRaw, `undo: ${JSON.stringify(raw)}`);
      const edited = await ctx.invoke("set_history_raw", { id: entry.id, raw: false });
      expect(!edited.showRaw, `redo: ${JSON.stringify(edited)}`);
      await ctx.settings({ ai: { cleanup: "none" } });
    },
  },
  {
    ids: ["HIS-07", "STA-01", "STA-05"],
    name: "auto-delete keeps the stats; Insights reads them",
    async run(ctx) {
      const before = await ctx.invoke("get_insights");
      expect(
        before.totalWords > 0 && before.days.length > 0 && before.topApp,
        `insights: ${JSON.stringify(before).slice(0, 300)}`,
      );
      const settings = await ctx.invoke("set_history_storage", {
        storage: "autoDelete",
        confirmation: null,
      });
      try {
        expect(settings.privacy.history === "autoDelete", `storage: ${settings.privacy.history}`);
        await ctx.dictate("hello", { hold: 4.5 });
        const after = await ctx.invoke("get_insights");
        expect(
          after.totalWords > before.totalWords,
          `words ${before.totalWords} → ${after.totalWords}`,
        );
      } finally {
        await ctx.invoke("set_history_storage", { storage: "local", confirmation: null });
      }
    },
  },
  {
    ids: ["LANG-01", "LANG-02", "LANG-05"],
    name: "languages: the empty-selection error, French among two, the active language",
    async run(ctx) {
      try {
        let message = "";
        await languages(ctx, []).catch((error) => (message = error.message));
        expectIncludes(message, "Please select at least one language or turn on auto-detect");
        await languages(ctx, ["en-US", "fr"]);
        await ctx.settings({ ai: { cleanup: "none" } });
        expectIncludes(await ctx.dictate("french", { hold: 6 }), "Bonjour");
        const active = await ctx.invoke("set_active_language", { id: "fr" });
        expect(active.active === "fr", `active: ${JSON.stringify(active)}`);
      } finally {
        await ctx.invoke("set_active_language", { id: null }).catch(() => {});
        await languages(ctx, ["en-US"]);
      }
    },
  },
  {
    ids: ["LANG-03"],
    name: "English (UK) spells the British way",
    async run(ctx) {
      try {
        await languages(ctx, ["en-GB"]);
        await ctx.settings({ ai: { cleanup: "none" } });
        const text = await ctx.dictate("favourite", { hold: 5 });
        expectIncludes(text, "favourite colour");
        expectIncludes(text, "centre");
      } finally {
        await languages(ctx, ["en-US"]);
      }
    },
  },
  {
    ids: ["LANG-04"],
    name: "speech in another language offers to add it",
    async run(ctx) {
      await languages(ctx, ["en-US"]);
      await ctx.settings({ notices: { languageCardDay: "", languageCardOff: false } });
      const since = await ctx.cursor();
      await ctx.dictate("french", { hold: 6 });
      await ctx.waitNotice(since, /Speaking a different language\? That sounded like French/);
    },
  },
  {
    ids: ["LANG-06", "NOT-01"],
    name: "the interface language reaches Rust's notices; Suggestions off drops the tip",
    async run(ctx) {
      await ctx.source("hello");
      try {
        await ctx.settings({ app: { language: "de" } });
        let since = await ctx.cursor();
        ctx.helper("hold", "fn", 0.25);
        const tip = await ctx.waitFor("a tip", async () => (await ctx.notices(since))[0], 5_000);
        const key = ctx.pushToTalkLabel;
        expect(!tip.includes(`Hold ${key}`) && tip.includes(key), `tip: ${tip}`);
        await ctx.settings({ app: { language: "system" }, notifications: { suggestions: false } });
        await ctx.waitIdle();
        since = await ctx.cursor();
        ctx.helper("hold", "fn", 0.25);
        await sleep(2000);
        const notices = await ctx.notices(since);
        expect(notices.length === 0, `notices with Suggestions off: ${notices}`);
      } finally {
        await ctx.settings({ app: { language: "system" }, notifications: { suggestions: true } });
      }
    },
  },
  {
    ids: ["PRIV-02"],
    name: "an API key goes to the keychain only",
    async run(ctx) {
      const key = `gsk_shrimp_regression_${process.pid}`;
      try {
        expect(
          (await ctx.invoke("set_api_key", { account: "groq", key })).groq,
          "the key wasn't stored",
        );
        await sleep(500);
        for (const file of ["settings.json", "shrimp.db", "shrimp.db-wal"]) {
          const path = join(DATA_DIR, file);
          if (existsSync(path)) expect(!readFileSync(path).includes(key), `the key is in ${file}`);
        }
        const logs = await ctx.logs(0);
        expect(!logs.some((line) => line.includes(key)), "the key is in the logs");
      } finally {
        await ctx.invoke("delete_api_key", { account: "groq" });
      }
      expect(!(await ctx.invoke("get_api_keys")).groq, "the key wasn't removed");
    },
  },
  {
    ids: ["PRIV-03"],
    name: "privacy mode shows on the Flow Bar",
    async run(ctx) {
      await ctx.settings({ privacy: { privacyMode: true } });
      try {
        await ctx.waitFor(
          "the privacy label",
          async () =>
            (await ctx.labels(".pill", "flow-bar")).some((label) => /Privacy mode on/.test(label)),
          5_000,
        );
      } finally {
        await ctx.settings({ privacy: { privacyMode: false } });
      }
    },
  },
  {
    ids: ["APP-01", "APP-02", "APP-04"],
    name: "the main window: sidebar, close to the tray, a second launch brings it back",
    async run(ctx) {
      const nav = await ctx.labels("nav a, nav button", "main");
      for (const page of [
        "Home",
        "Insights",
        "Dictionary",
        "Snippets",
        "Style",
        "Transforms",
        "Settings",
        "Help",
      ]) {
        expect(
          nav.some((label) => label.includes(page)),
          `sidebar lacks ${page}: ${nav}`,
        );
      }
      const pid = ctx.appPid();
      await ctx.app.send("manage_window", { operation: "close", window_label: "main" });
      await ctx.waitFor(
        "the window to close",
        async () => !(await ctx.windowVisible("main")),
        5_000,
      );
      expect(ctx.appPid() === pid, "closing the window quit the app");
      ctx.launchAgain();
      await ctx.waitFor("the window to come back", async () => ctx.windowVisible("main"), 10_000);
      expect(ctx.appPid() === pid, "a second launch started another process");
      ctx.focusTarget();
    },
  },
];
