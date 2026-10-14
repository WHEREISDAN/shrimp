// The items the phase 6 parity audit added or changed (AUD-01).

import { execFileSync } from "node:child_process";

import { TARGET, TARGET_NAME, expect, expectIncludes, sleep, soft } from "../context.mjs";
import { withOllama } from "./editing.mjs";

const SHRIMP = "com.shrimp.desktop.dev";
const RAMBLING =
  "So basically the launch is going to be delayed because there is a payment bug and it needs a security review before we can ship it.";

function edit(ctx, text) {
  return ctx.invoke("dev_edit_text", { text, appId: TARGET });
}

async function navigate(ctx, section) {
  await ctx.app.send("manage_ipc", {
    action: "emit",
    event: "app://navigate",
    payload: section,
    window_label: "main",
  });
  await sleep(600);
}

function dockType() {
  const out = execFileSync(
    "lsappinfo",
    ["info", "-only", "applicationtype", "-app", "Shrimp Dev"],
    {
      encoding: "utf8",
    },
  );
  return /"ApplicationType"="(\w+)"/.exec(out)?.[1];
}

export const audit = [
  {
    ids: ["FMT-15"],
    name: "formatting per app: unformatted, all lowercase, proper-noun capitals",
    async run(ctx) {
      const set = (format) =>
        ctx.settings({ appFormats: [{ appId: TARGET, appName: TARGET_NAME, format }] });
      try {
        await ctx.settings({ ai: { cleanup: "none" } });
        await set("unformatted");
        const raw = await edit(ctx, "onto the server comma finally new line thanks");
        expect(
          raw.plain === "onto the server comma finally\nThanks",
          `unformatted: ${JSON.stringify(raw)}`,
        );
        await set("lowercase");
        const lower = await edit(ctx, "Thanks Siobhan. I will send it.");
        expect(lower.plain === "thanks siobhan. i will send it.", `lowercase: ${lower.plain}`);
        await set("properNouns");
        const nouns = await edit(ctx, "Please ask Siobhan. The meeting is on Monday.");
        expectIncludes(nouns.plain, "Siobhan");
        expectIncludes(nouns.plain, "the meeting");
        const apps = await ctx.invoke("get_format_apps");
        expect(
          JSON.stringify(apps).includes(TARGET_NAME),
          `apps offered: ${JSON.stringify(apps).slice(0, 300)}`,
        );
      } finally {
        await ctx.settings({ appFormats: [] });
      }
    },
  },
  {
    ids: ["FMT-05", "FMT-06"],
    name: "spoken em-dash variants and “start a new paragraph”",
    async run(ctx) {
      await ctx.settings({ ai: { cleanup: "none" } });
      expectIncludes((await edit(ctx, "wait em-dash really")).plain, "wait—really");
      const paragraph = await edit(
        ctx,
        "Thanks for coming. Start a new paragraph. The slides follow.",
      );
      expect(
        /coming\.\n\nThe slides/.test(paragraph.plain),
        `paragraph: ${JSON.stringify(paragraph.plain)}`,
      );
    },
  },
  {
    ids: ["CORE-17"],
    name: "the first “press enter” asks; Great turns the command on and presses Enter",
    async run(ctx) {
      await ctx.settings({
        ai: { cleanup: "none" },
        experimental: { pressEnterCommand: false, pressEnterPromptSeen: false },
      });
      try {
        const since = await ctx.cursor();
        const text = await ctx.dictate("press-enter", { hold: 4 });
        expect(/^ship it now\.?$/i.test(text), `inserted ${JSON.stringify(text)}`);
        await ctx.waitNotice(since, /left out “press enter”/);
        await ctx.invoke("run_notice_action", { action: "enablePressEnter" });
        await ctx.waitFor("Enter", async () => ctx.text().endsWith("\n"), 4_000);
        expect(
          (await ctx.invoke("get_settings")).experimental.pressEnterCommand,
          "the command stayed off",
        );
        const again = await ctx.cursor();
        await ctx.dictate("hello", { hold: 4.5 });
        expect(
          !(await ctx.notices(again)).some((notice) => /press enter/.test(notice)),
          "asked twice",
        );
      } finally {
        await ctx.settings({ experimental: { pressEnterCommand: false } });
      }
    },
  },
  {
    ids: ["SNP-01"],
    name: "a snippet trigger can't repeat a dictionary word",
    async run(ctx) {
      const word = await ctx.invoke("add_dictionary_entry", { input: { term: "Shrimpish" } });
      try {
        let message = "";
        await ctx
          .invoke("add_snippet", { input: { trigger: "shrimpish", expansion: "x" } })
          .catch((error) => (message = error.message));
        expectIncludes(message, "already exists in your personal dictionary");
      } finally {
        await ctx.invoke("delete_dictionary_entries", { ids: [word.id] });
      }
    },
  },
  {
    ids: ["HIS-10", "CORE-11"],
    name: "a cancelled dictation is kept in History and can be recovered",
    async run(ctx) {
      await ctx.settings({
        ai: { cleanup: "none" },
        notices: { cancelRecoverCount: 0, cancelRecoverDay: "" },
      });
      await ctx.source("padded");
      ctx.setText("");
      ctx.focusTarget();
      const since = await ctx.cursor();
      ctx.helper("down", "fn");
      await sleep(3000);
      ctx.helper("tap", "esc");
      ctx.helper("up", "fn");
      await ctx.waitIdle();
      await ctx.waitNotice(since, /You can recover your cancelled transcript/);
      const [entry] = (await ctx.invoke("list_history", { query: "" })).entries;
      expect(
        entry?.status === "cancelled" && entry.retry === "recover",
        `entry: ${JSON.stringify(entry)}`,
      );
      await ctx.invoke("retry_history", { id: entry.id });
      await ctx.waitFor(
        "the recovered text",
        async () =>
          (await ctx.invoke("list_history", { query: "" })).entries
            .find((item) => item.id === entry.id)
            ?.finalText?.includes("Hello"),
        30_000,
      );
      expect(ctx.text() === "", `Recover pasted: ${JSON.stringify(ctx.text())}`);
    },
  },
  {
    ids: ["HIS-09", "STA-05"],
    name: "Delete all transcripts empties History and keeps the stats",
    async run(ctx) {
      await ctx.dictate("hello", { hold: 4.5 });
      const before = await ctx.invoke("get_insights");
      await ctx.invoke("delete_all_history");
      const list = await ctx.invoke("list_history", { query: "" });
      expect(list.entries.length === 0, `left ${list.entries.length} entries`);
      const after = await ctx.invoke("get_insights");
      expect(
        after.totalWords === before.totalWords,
        `words ${before.totalWords} → ${after.totalWords}`,
      );
    },
  },
  {
    ids: ["STA-06", "STA-02"],
    name: "Insights counts words per usage category; the greeting's shortcut",
    async run(ctx) {
      await ctx.invoke("dev_set_app_category", { appId: TARGET, category: "email" });
      try {
        const emails = async () =>
          (await ctx.invoke("get_insights")).usage.find((row) => row.category === "emails")
            ?.words ?? 0;
        const before = await emails();
        await ctx.dictate("hello", { hold: 4.5 });
        const { usage, totalWords } = await ctx.invoke("get_insights");
        expect(usage.length === 6, `categories: ${usage.map((row) => row.category)}`);
        expect((await emails()) > before, "the email words didn't grow");
        await navigate(ctx, "home");
        const greeting = (await ctx.app.readText(".greeting")).join(" ");
        expect(
          greeting.includes(`Hold ${ctx.pushToTalkLabel}`) === totalWords < 2000,
          `${totalWords} words, greeting: ${greeting}`,
        );
      } finally {
        await ctx.invoke("dev_set_app_category", { appId: TARGET, category: null });
        ctx.focusTarget();
      }
    },
  },
  {
    ids: ["MIC-08"],
    name: "the Bluetooth advice shows at most once an hour",
    async run(ctx) {
      const pods = [
        {
          id: "pods",
          name: "AirPods Pro",
          kind: "bluetooth",
          behaviour: { type: "wav", path: ctx.fixture("hello") },
        },
      ];
      try {
        await ctx.settings({ notices: { bluetoothAdviceAt: 0, bluetoothAdviceOff: false } });
        await ctx.invoke("dev_set_audio_source", { spec: "mic" });
        await ctx.invoke("dev_set_microphones", {
          devices: pods,
          default: "pods",
          lidClosed: false,
        });
        let since = await ctx.cursor();
        await ctx.dictate("hello", { hold: 4, keepText: true, source: false });
        await ctx.waitNotice(since, /Bluetooth microphones like AirPods Pro/);
        since = await ctx.cursor();
        await ctx.dictate("hello", { hold: 4, keepText: true, source: false });
        await sleep(1000);
        expect(
          !(await ctx.notices(since)).some((notice) => /Bluetooth/.test(notice)),
          "shown twice within an hour",
        );
      } finally {
        await ctx.invoke("dev_set_microphones", { devices: null, default: null, lidClosed: false });
        await ctx.settings({ microphone: { selected: null, ranking: [] } });
        await ctx.source("hello");
      }
    },
  },
  {
    ids: ["SND-01"],
    name: "the paste cue follows a paste, not a copy",
    async run(ctx) {
      let since = await ctx.cursor();
      await ctx.dictate("hello", { hold: 4.5 });
      expect(
        (await ctx.logs(since)).some((line) => /^Paste cue started/.test(line)),
        "no paste cue",
      );
      ctx.focusTarget("table");
      since = await ctx.cursor();
      ctx.helper("hold", "fn", 4.5);
      await ctx.waitIdle();
      await ctx.waitNotice(since, /No text field is focused/);
      expect(
        !(await ctx.logs(since)).some((line) => /^Paste cue started/.test(line)),
        "a paste cue after a copy",
      );
      ctx.focusTarget("text");
    },
  },
  {
    ids: ["KEYS-11", "DIC-04"],
    name: "the Shortcuts dialog's double-tap hint; the dictionary's Sort by",
    async run(ctx) {
      await navigate(ctx, "shortcuts");
      const dialog = (await ctx.app.readText("[role=dialog]")).join(" ");
      expectIncludes(dialog, `Or double-tap ${ctx.pushToTalkLabel}`, "Shortcuts dialog");
      await ctx.app.send("manage_ipc", {
        action: "emit",
        event: "app://navigate",
        payload: "home",
        window_label: "main",
      });
      const words = [];
      try {
        for (const term of ["Zebrafish", "Aardvark"])
          words.push((await ctx.invoke("add_dictionary_entry", { input: { term } })).id);
        const order = async (sort) => {
          await ctx.settings({ dictionary: { sort } });
          await navigate(ctx, "dictionary");
          const titles = await ctx.app.readText(".list-title");
          return titles.filter((title) => ["Zebrafish", "Aardvark"].includes(title));
        };
        expect(
          JSON.stringify(await order("alphabetical")) === '["Aardvark","Zebrafish"]',
          "alphabetical",
        );
        expect(
          JSON.stringify(await order("oldest")) === '["Zebrafish","Aardvark"]',
          "oldest first",
        );
      } finally {
        await ctx.settings({ dictionary: { sort: "starred" } });
        if (words.length) await ctx.invoke("delete_dictionary_entries", { ids: words });
        ctx.focusTarget();
      }
    },
  },
  {
    ids: ["TRF-07", "TRF-04"],
    name: "the Flow Bar wand polishes the selection; Turn to list re-transforms it",
    async run(ctx) {
      await withOllama(ctx);
      await ctx.settings({ transforms: { enabled: true, wand: "polish" } });
      try {
        await ctx.app.send("manage_ipc", {
          action: "emit",
          event: "flow-bar://hover",
          payload: true,
          window_label: "flow-bar",
        });
        await sleep(500);
        const labels = await ctx.labels(".wand button", "flow-bar");
        expect(
          labels.some((label) => /Run Polish/.test(label)),
          `wand: ${JSON.stringify(labels)}`,
        );
        await ctx.app.send("manage_ipc", {
          action: "emit",
          event: "flow-bar://hover",
          payload: false,
          window_label: "flow-bar",
        });
        ctx.setText(RAMBLING);
        ctx.select(0, RAMBLING.length);
        await ctx.invoke("flow_bar_wand", { action: "run" });
        await ctx.waitFor("the polished text", async () => ctx.text() !== RAMBLING, 60_000);
        expect(ctx.frontmost() === TARGET, "the wand took focus");
        const polished = ctx.text();
        await ctx.invoke("transform_action", { action: { retransform: "turnToList" } });
        await ctx.waitFor("the list", async () => ctx.text() !== polished, 60_000);
        soft(/^\s*([-•*]|1[.)])/m.test(ctx.text()), `Turn to list: ${ctx.text()}`);
        await ctx.invoke("transform_action", { action: "undo" });
        await ctx.waitFor("the original", async () => ctx.text() === RAMBLING, 10_000);
      } finally {
        await ctx.settings({ transforms: { enabled: false } });
      }
    },
  },
  {
    ids: ["TRF-05"],
    name: "Auto Apply says it polished the dictation, and Reveal edit shows the changes",
    async run(ctx) {
      await withOllama(ctx, "none");
      await ctx.settings({ transforms: { enabled: true, autoApply: "polish" } });
      try {
        const since = await ctx.cursor();
        await ctx.dictate("filler", { hold: 5 });
        await ctx.waitNotice(since, /We polished your dictation/, 30_000);
        await ctx.arm("transform://changes", 30_000);
        const captured = await ctx.capturedCursor();
        await ctx.invoke("run_notice_action", { action: "revealEdit" });
        await ctx.waitFor(
          "the changes view",
          async () => (await ctx.captured("transform://changes", captured)).length,
          10_000,
        );
      } finally {
        await ctx.settings({ transforms: { enabled: false, autoApply: null } });
      }
    },
  },
  {
    ids: ["APP-10"],
    name: "the macOS menu bar",
    platforms: ["darwin"],
    async run(ctx) {
      const menus = ctx.helper("menubar", SHRIMP);
      const titles = (name) =>
        menus.find((menu) => menu.title === name)?.items.map((item) => item.title) ?? [];
      for (const item of ["About Shrimp", "Settings…", "Quit Shrimp"])
        expect(titles("Shrimp Dev").includes(item), `app menu lacks ${item}`);
      for (const item of ["Undo", "Paste", "Select All"])
        expect(titles("Edit").includes(item), `Edit lacks ${item}`);
      expect(
        titles("Dictation").includes("Copy last transcript"),
        `Dictation: ${titles("Dictation")}`,
      );
      for (const item of ["Shortcuts", "Shrimp Help"])
        expect(titles("Help").includes(item), `Help lacks ${item}`);
      ctx.helper("press-menu", SHRIMP, "Shrimp Dev", "Settings…");
      await sleep(800);
      expectIncludes(
        (await ctx.labels("[role=tab]", "main")).join(" "),
        "General",
        "settings tabs",
      );
      ctx.focusTarget();
    },
  },
  {
    ids: ["APP-03"],
    name: "the tray menu; on Windows a left click on the icon opens the main window",
    async run(ctx) {
      const tray = ctx.trayItems().map((item) => item.title);
      for (const item of [
        "Open Shrimp",
        "Shortcuts…",
        "Paste last transcript",
        "Copy last transcript",
        "Restart Shrimp",
        "Quit Shrimp",
      ]) {
        expect(
          tray.some((title) => title.startsWith(item)),
          `tray lacks ${item}: ${tray}`,
        );
      }
      if (ctx.trayClickOpensWindow) {
        await ctx.app.send("manage_window", { operation: "close", window_label: "main" });
        await ctx.waitFor("the closed window", async () => !(await ctx.windowVisible("main")));
        ctx.trayClick("left");
        await ctx.waitFor("the main window", async () => ctx.windowVisible("main"), 5_000);
      }
      ctx.focusTarget();
    },
  },
  {
    ids: ["APP-07"],
    name: "with Show app in Dock off, the Dock icon stays only while the window is open",
    platforms: ["darwin"],
    async run(ctx) {
      await ctx.settings({ app: { showInDock: false } });
      try {
        execFileSync("open", [ctx.bundle]);
        await ctx.waitFor("the window", async () => ctx.windowVisible("main"), 5_000);
        await ctx.waitFor("a Dock icon", async () => dockType() === "Foreground", 5_000);
        await ctx.app.send("manage_window", { operation: "close", window_label: "main" });
        await ctx.waitFor("no Dock icon", async () => dockType() === "UIElement", 5_000);
      } finally {
        await ctx.settings({ app: { showInDock: true } });
        ctx.focusTarget();
      }
    },
  },
];
