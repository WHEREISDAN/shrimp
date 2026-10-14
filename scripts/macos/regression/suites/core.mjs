// Core dictation, insertion, Flow Bar and sounds (phase 1 and its phase 3–4 additions).

import { execFileSync } from "node:child_process";
import { expect, expectIncludes, sleep } from "../context.mjs";

const HELLO = "Hello from Shrimp";

/** Raw transcripts, so the checks don't depend on a model. */
async function raw(ctx) {
  await ctx.settings({ ai: { cleanup: "none" } });
}

export const core = [
  {
    ids: ["ENG-01", "CORE-01", "INS-01"],
    name: "holding fn dictates the WAV into the focused field",
    async run(ctx) {
      await raw(ctx);
      const text = await ctx.dictate("hello", { hold: 4.5 });
      expectIncludes(text, HELLO);
      expectIncludes(text, "local dictation");
    },
  },
  {
    ids: ["INS-07"],
    name: "one Cmd+Z removes the inserted text",
    async run(ctx) {
      expectIncludes(ctx.text(), HELLO);
      ctx.helper("tap", "cmd+z");
      await sleep(300);
      expect(ctx.text() === "", `after Cmd+Z: ${JSON.stringify(ctx.text())}`);
    },
  },
  {
    ids: ["INS-02"],
    name: "the clipboard comes back after the paste",
    async run(ctx) {
      ctx.setClipboard("regression sentinel");
      const text = await ctx.dictate("hello", { hold: 4.5 });
      expectIncludes(text, HELLO);
      await sleep(500);
      expect(ctx.clipboard() === "regression sentinel", `clipboard: ${ctx.clipboard()}`);
    },
  },
  {
    ids: ["ENG-03"],
    name: "silence inserts nothing",
    async run(ctx) {
      const since = await ctx.cursor();
      const text = await ctx.dictate("silence", { hold: 3.5 });
      expect(text === "", `inserted ${JSON.stringify(text)}`);
      await ctx.waitNotice(since, /No speech detected/);
    },
  },
  {
    ids: ["CORE-02"],
    name: "fn alone is the push-to-talk default",
    async run(ctx) {
      const { rows } = await ctx.invoke("get_shortcuts");
      const push = rows.find((row) => row.action === "pushToTalk");
      expect(push?.shortcuts?.[0] === "fn", `push to talk: ${JSON.stringify(push?.shortcuts)}`);
    },
  },
  {
    ids: ["CORE-03", "BAR-03"],
    name: "a short tap inserts nothing and shows the tip",
    async run(ctx) {
      await ctx.source("hello");
      ctx.setText("");
      const since = await ctx.cursor();
      ctx.helper("hold", "fn", 0.25);
      await ctx.waitNotice(since, /Hold fn to dictate/);
      const shown = await ctx.app.readText(".notice-message", "flow-bar");
      expect(
        shown.some((text) => /Hold fn to dictate/.test(text)),
        `Flow Bar notices: ${JSON.stringify(shown)}`,
      );
      await ctx.waitIdle();
      expect(ctx.text() === "", `inserted ${JSON.stringify(ctx.text())}`);
    },
  },
  {
    ids: ["CORE-04"],
    name: "fn Space records hands-free until pressed again",
    async run(ctx) {
      await ctx.source("padded");
      ctx.setText("");
      ctx.helper("tap", "fn+space");
      await sleep(1500);
      const state = await ctx.invoke("get_dictation_state");
      expect(state.phase === "recording", `after release: ${JSON.stringify(state)}`);
      await sleep(1500);
      ctx.helper("tap", "fn+space");
      await ctx.waitIdle();
      await ctx.waitFor("the paste", async () => ctx.text().includes("Hello"), 4000);
    },
  },
  {
    ids: ["CORE-05"],
    name: "a double tap locks hands-free; a third quick tap cancels",
    async run(ctx) {
      await ctx.source("padded");
      ctx.setText("");
      ctx.helper("tap", "fn");
      await sleep(120);
      ctx.helper("tap", "fn");
      await sleep(1500);
      const locked = await ctx.invoke("get_dictation_state");
      expect(locked.phase === "recording", `after the double tap: ${JSON.stringify(locked)}`);
      await sleep(1500);
      ctx.helper("tap", "fn");
      await ctx.waitIdle();
      await ctx.waitFor("the paste", async () => ctx.text().includes("Hello"), 4000);

      ctx.setText("");
      const since = await ctx.cursor();
      for (let tap = 0; tap < 3; tap++) {
        ctx.helper("tap", "fn");
        await sleep(120);
      }
      await ctx.waitIdle();
      await sleep(1000);
      expect(ctx.text() === "", `the third tap inserted ${JSON.stringify(ctx.text())}`);
      expect(
        (await ctx.logs(since)).some((line) => /cancel/i.test(line)),
        "no cancellation logged",
      );
    },
  },
  {
    ids: ["CORE-06", "CORE-11"],
    name: "Esc cancels; the notice's Undo inserts the cancelled audio",
    async run(ctx) {
      await ctx.source("padded");
      ctx.setText("");
      const since = await ctx.cursor();
      ctx.helper("down", "fn");
      await sleep(3500);
      ctx.helper("tap", "esc");
      ctx.helper("up", "fn");
      await ctx.waitIdle();
      await ctx.waitNotice(since, /Dictation cancelled/);
      expect(
        ctx.text() === "",
        `Esc reached the field or text was inserted: ${JSON.stringify(ctx.text())}`,
      );
      await ctx.invoke("run_notice_action", { action: "undoCancel" });
      await ctx.waitFor("the undone dictation", async () => ctx.text().includes("Hello"), 15_000);
    },
  },
  {
    ids: ["CORE-07"],
    name: "a press while processing is rejected",
    async run(ctx) {
      await ctx.source("hello");
      ctx.setText("");
      const since = await ctx.cursor();
      ctx.helper("hold", "fn", 3);
      ctx.helper("tap", "fn");
      await ctx.waitNotice(since, /Transcript currently processing/);
      await ctx.waitIdle();
      await sleep(800);
      const pasted = ctx.text().split("Hello").length - 1;
      expect(pasted === 1, `inserted ${pasted} transcripts`);
    },
  },
  {
    ids: ["CORE-09", "BAR-01", "BAR-02"],
    name: "the Flow Bar starts, shows recording and stops a dictation",
    async run(ctx) {
      await ctx.source("padded");
      ctx.setText("");
      ctx.focusTarget();
      await ctx.invoke("flow_bar_control", { control: "start" });
      await sleep(800);
      const labels = await ctx.labels(".pill", "flow-bar");
      expect(labels.includes("Recording"), `pill labels: ${JSON.stringify(labels)}`);
      expect(ctx.frontmost() === "com.shrimp.regression.target", "the Flow Bar took focus");
      await sleep(2000);
      await ctx.invoke("flow_bar_control", { control: "stop" });
      await ctx.waitIdle();
      await ctx.waitFor("the paste", async () => ctx.text().includes("Hello"), 4000);

      ctx.setText("");
      await ctx.invoke("flow_bar_control", { control: "start" });
      await sleep(200);
      await ctx.invoke("flow_bar_control", { control: "stop" });
      await ctx.waitIdle();
      await sleep(1000);
      expect(ctx.text() === "", `a quick second click inserted ${JSON.stringify(ctx.text())}`);
    },
  },
  {
    ids: ["CORE-12"],
    name: "another key early in a hold cancels it",
    async run(ctx) {
      await ctx.source("padded");
      ctx.setText("");
      const since = await ctx.cursor();
      ctx.helper("down", "fn");
      await sleep(400);
      ctx.helper("tap", "f");
      await sleep(600);
      ctx.helper("up", "fn");
      await ctx.waitNotice(since, /another shortcut was pressed/);
      await ctx.waitIdle();
      expect(!ctx.text().includes("Hello"), `inserted ${JSON.stringify(ctx.text())}`);
    },
  },
  {
    ids: ["CORE-13", "KEYS-10"],
    name: "Mouse 4 bound to push to talk dictates and is saved",
    async run(ctx) {
      await ctx.settings({ shortcuts: { bindings: { pushToTalk: ["Fn", "Mouse4"] } } });
      try {
        const saved = ctx.readSettingsFile();
        expect(
          JSON.stringify(saved.shortcuts?.bindings?.pushToTalk) === '["Fn","Mouse4"]',
          `settings.json: ${JSON.stringify(saved.shortcuts)}`,
        );
        const text = await ctx.dictate("hello", { hold: 4.5, button: "mouse4" });
        expectIncludes(text, HELLO);
      } finally {
        await ctx.settings({ shortcuts: { bindings: { pushToTalk: null } } });
      }
    },
  },
  {
    ids: ["CORE-15"],
    name: "the Press Enter shortcut sends Enter",
    async run(ctx) {
      await ctx.settings({ shortcuts: { bindings: { pressEnter: ["Opt+Return"] } } });
      try {
        ctx.setText("abc");
        ctx.helper("tap", "opt+return");
        await sleep(400);
        expect(ctx.text() === "abc\n", `after ⌥↩: ${JSON.stringify(ctx.text())}`);
      } finally {
        await ctx.settings({ shortcuts: { bindings: { pressEnter: null } } });
      }
    },
  },
  {
    ids: ["CORE-16"],
    name: "ending with “press enter” presses Enter after the paste",
    async run(ctx) {
      await ctx.settings({ experimental: { pressEnterCommand: true } });
      try {
        const text = await ctx.dictate("press-enter", { hold: 4 });
        expect(/ship it now/i.test(text), `inserted ${JSON.stringify(text)}`);
        expect(!/press enter/i.test(text), `the phrase stayed: ${JSON.stringify(text)}`);
        expect(text.endsWith("\n"), `no Enter: ${JSON.stringify(text)}`);
      } finally {
        await ctx.settings({ experimental: { pressEnterCommand: false } });
      }
    },
  },
  {
    ids: ["INS-08", "FMT-06"],
    name: "spoken line breaks become new lines and paragraphs",
    async run(ctx) {
      const text = await ctx.dictate("lines", { hold: 7 });
      expect(/line\.?\nSecond/i.test(text), `no line break: ${JSON.stringify(text)}`);
      expect(/\n\nThird/i.test(text), `no paragraph: ${JSON.stringify(text)}`);
    },
  },
  {
    ids: ["INS-05", "INS-06"],
    name: "⌃⌘V pastes and ⌃⌘C copies the last transcript",
    async run(ctx) {
      await ctx.dictate("hello", { hold: 4.5 });
      ctx.setText("");
      ctx.helper("tap", "ctrl+cmd+v");
      await ctx.waitFor("the pasted transcript", async () => ctx.text().includes("Hello"), 6000);
      await sleep(800);
      ctx.setClipboard("before copy last");
      ctx.helper("tap", "ctrl+cmd+c");
      await ctx.waitFor(
        "the copied transcript",
        async () => (ctx.clipboard() ?? "").includes("Hello"),
        4000,
      );
      expect(!ctx.text().includes("before copy last"), "⌃⌘C reached the field");
    },
  },
  {
    ids: ["INS-04"],
    name: "with no text field focused the transcript stays on the clipboard",
    async run(ctx) {
      await ctx.source("hello");
      ctx.focusTarget("table");
      const since = await ctx.cursor();
      ctx.helper("hold", "fn", 4.5);
      await ctx.waitIdle();
      await ctx.waitNotice(since, /No text field is focused/);
      expectIncludes(ctx.clipboard(), HELLO, "clipboard");
      ctx.focusTarget("text");
    },
  },
  {
    ids: ["ENG-07", "FMT-10"],
    name: "a missing key or a dead provider gives a notice and the raw transcript",
    async run(ctx) {
      try {
        await ctx.settings({ ai: { cleanup: "light", provider: "anthropic" } });
        if (!(await ctx.invoke("get_api_keys")).anthropic) {
          const since = await ctx.cursor();
          expectIncludes(await ctx.dictate("hello", { hold: 4.5 }), HELLO);
          await ctx.waitLog(since, /AI edit not used: Llm\(Anthropic, MissingKey\)/);
          // Shown once per provider per session.
          await ctx.waitNotice(0, /AI edits need your Anthropic API key/);
        } else {
          ctx.note("ENG-07: an Anthropic key is set, so the missing-key path wasn't run");
        }
        await ctx.settings({ ai: { provider: "ollama", ollamaUrl: "http://127.0.0.1:9" } });
        const since = await ctx.cursor();
        expectIncludes(await ctx.dictate("hello", { hold: 4.5 }), HELLO);
        await ctx.waitNotice(since, /Ollama isn't running/);
        const [entry] = (await ctx.invoke("list_history", { query: "" })).entries;
        expect(entry && !entry.formatted, `history entry: ${JSON.stringify(entry)}`);
      } finally {
        await ctx.settings({ ai: { cleanup: "none", ollamaUrl: "http://localhost:11434" } });
      }
    },
  },
  {
    ids: ["SND-01", "SND-02"],
    name: "dictation cues play, and stop when switched off",
    async run(ctx) {
      let since = await ctx.cursor();
      await ctx.dictate("hello", { hold: 4.5 });
      const cues = (await ctx.logs(since)).filter((line) => /cue started/.test(line));
      expect(
        cues.some((line) => /^Start/.test(line)),
        `cues: ${JSON.stringify(cues)}`,
      );
      expect(
        cues.some((line) => /^Stop/.test(line)),
        `cues: ${JSON.stringify(cues)}`,
      );
      await ctx.settings({ sounds: { dictation: false } });
      try {
        since = await ctx.cursor();
        await ctx.dictate("hello", { hold: 4.5 });
        const muted = (await ctx.logs(since)).filter((line) =>
          /^(Start|Stop) cue started/.test(line),
        );
        expect(muted.length === 0, `cues played: ${JSON.stringify(muted)}`);
      } finally {
        await ctx.settings({ sounds: { dictation: true } });
      }
    },
  },
  {
    ids: ["PRIV-01"],
    name: "a default dictation opens no network socket",
    async run(ctx) {
      await ctx.source("padded");
      ctx.setText("");
      ctx.focusTarget();
      const pid = ctx.appPid();
      const samples = [];
      await ctx.invoke("flow_bar_control", { control: "start" });
      for (let sample = 0; sample < 20; sample++) {
        samples.push(lsof(pid));
        await sleep(150);
      }
      await ctx.invoke("flow_bar_control", { control: "stop" });
      await ctx.waitIdle();
      for (let sample = 0; sample < 10; sample++) {
        samples.push(lsof(pid));
        await sleep(100);
      }
      const open = samples.filter(Boolean);
      expect(open.length === 0, `sockets: ${open[0]}`);
    },
  },
  {
    ids: ["BAR-04", "BAR-06"],
    name: "the resting bar hides when not always shown, and for an hour from its menu",
    async run(ctx) {
      await ctx.settings({ flowBar: { alwaysShow: false } });
      try {
        await ctx.waitFor(
          "the bar to hide",
          async () => !(await ctx.windowVisible("flow-bar")),
          5000,
        );
      } finally {
        await ctx.settings({ flowBar: { alwaysShow: true } });
      }
      await ctx.waitFor("the bar to come back", async () => ctx.windowVisible("flow-bar"), 5000);
      const since = await ctx.cursor();
      await ctx.invoke("flow_bar_action", { action: "hide" });
      try {
        await ctx.waitNotice(since, /Flow Bar hidden for 1 hour/);
        const { hiddenUntil } = (await ctx.invoke("get_settings")).flowBar;
        expect(hiddenUntil * 1000 > Date.now() + 50 * 60_000, `hidden until ${hiddenUntil}`);
      } finally {
        await ctx.invoke("run_notice_action", { action: "showFlowBar" });
      }
      await ctx.waitFor("the bar after Undo", async () => ctx.windowVisible("flow-bar"), 5000);
    },
  },
  {
    ids: ["BAR-05"],
    name: "the bar docks to the left edge and back",
    async run(ctx) {
      const position = async () =>
        (await ctx.app.send("get_app_info", {})).windows.find(
          (window) => window.label === "flow-bar",
        ).position;
      const bottom = await position();
      await ctx.settings({ flowBar: { edge: "left", position: 0.5 } });
      try {
        await ctx.waitFor(
          "the bar on the left",
          async () => (await position()).x < bottom.x - 300,
          5000,
        );
      } finally {
        await ctx.settings({ flowBar: { edge: "bottom", position: 0.5 } });
      }
      await ctx.waitFor(
        "the bar at the bottom",
        async () => (await position()).y >= bottom.y - 5,
        5000,
      );
    },
  },
];

/** Internet sockets the process has open, or "" when there are none. */
function lsof(pid) {
  try {
    return execFileSync("lsof", ["-nP", "-a", "-i", "-p", String(pid)], {
      encoding: "utf8",
    }).trim();
  } catch {
    return "";
  }
}
