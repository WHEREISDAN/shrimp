// Command Mode and Transforms (phase 3), with the local Ollama as the model.

import { expect, sleep, soft } from "../context.mjs";
import { withOllama } from "./editing.mjs";

const ROUGH = "hey can u send me the doc by tmrw";
const RAMBLING =
  "So basically the launch is going to be delayed because there is a payment bug and it needs a security review before we can ship it.";

async function commandMode(ctx, on) {
  await ctx.settings({ experimental: { commandMode: on, commandModeIntroSeen: true } });
}

async function transforms(ctx, on) {
  await ctx.settings({ transforms: { enabled: on } });
}

/** Selects all of `text` in the target's text view. */
function selectAll(ctx, text) {
  ctx.setText(text);
  ctx.select(0, [...text].length);
}

export const commands = [
  {
    ids: ["CMD-06"],
    name: "Command Mode's shortcut while it is off explains how to turn it on",
    async run(ctx) {
      await commandMode(ctx, false);
      await ctx.source("instruction");
      ctx.setText("");
      const since = await ctx.cursor();
      ctx.helper("hold", "ctrl+fn", 1.5);
      await ctx.waitNotice(since, /Command mode is toggled off/);
      await ctx.waitIdle();
    },
  },
  {
    ids: ["CMD-01", "CMD-02"],
    name: "a spoken instruction rewrites the selection in place",
    async run(ctx) {
      await withOllama(ctx);
      await commandMode(ctx, true);
      try {
        await ctx.source("instruction");
        selectAll(ctx, ROUGH);
        ctx.setClipboard("command sentinel");
        ctx.helper("hold", "ctrl+fn", 3);
        await ctx.waitIdle(60_000);
        await ctx.waitFor("the rewrite", async () => ctx.text() !== ROUGH, 10_000);
        soft(/document|tomorrow/i.test(ctx.text()), `rewrite: ${ctx.text()}`);
        await sleep(800);
        expect(ctx.clipboard() === "command sentinel", `clipboard: ${ctx.clipboard()}`);
      } finally {
        await commandMode(ctx, false);
      }
    },
  },
  {
    ids: ["CMD-03"],
    name: "a question with no text field is answered in the Flow Bar",
    async run(ctx) {
      await withOllama(ctx);
      await commandMode(ctx, true);
      try {
        await ctx.source("question");
        ctx.focusTarget("table");
        await ctx.arm("command://answer", 120_000);
        const since = await ctx.capturedCursor();
        ctx.helper("hold", "ctrl+fn", 3);
        await ctx.waitIdle(60_000);
        const [answer] = await ctx.waitFor(
          "the answer",
          async () => {
            const events = await ctx.captured("command://answer", since);
            return events.length ? events.map((event) => event.payload) : null;
          },
          20_000,
        );
        soft(/canberra/i.test(JSON.stringify(answer)), `answer: ${JSON.stringify(answer)}`);
      } finally {
        ctx.focusTarget("text");
        await commandMode(ctx, false);
      }
    },
  },
  {
    ids: ["CMD-04"],
    name: "a double press locks Command Mode and Backspace cancels it",
    async run(ctx) {
      await commandMode(ctx, true);
      try {
        await ctx.source("padded");
        selectAll(ctx, ROUGH);
        ctx.helper("tap", "ctrl+fn");
        await sleep(120);
        ctx.helper("tap", "ctrl+fn");
        await sleep(1200);
        expect((await ctx.state()) === "recording", "not locked after the double press");
        ctx.helper("tap", "delete");
        await ctx.waitIdle();
        await sleep(500);
        expect(ctx.text() === ROUGH, `Backspace reached the field: ${JSON.stringify(ctx.text())}`);
      } finally {
        await commandMode(ctx, false);
      }
    },
  },
  {
    ids: ["TRF-06"],
    name: "a transform with nothing selected asks for a selection",
    async run(ctx) {
      await transforms(ctx, true);
      try {
        ctx.setText("");
        const since = await ctx.cursor();
        ctx.helper("tap", "opt+1");
        await ctx.waitNotice(since, /Select text to apply a transform/);
      } finally {
        await transforms(ctx, false);
      }
    },
  },
  {
    ids: ["TRF-01", "TRF-04"],
    name: "Opt+1 polishes the selection in place; Opt+O shows the changes",
    async run(ctx) {
      await withOllama(ctx);
      await transforms(ctx, true);
      try {
        selectAll(ctx, RAMBLING);
        ctx.setClipboard("transform sentinel");
        ctx.helper("tap", "opt+1");
        await ctx.waitFor("the polished text", async () => ctx.text() !== RAMBLING, 60_000);
        await ctx.arm("transform://changes", 60_000);
        const since = await ctx.capturedCursor();
        ctx.helper("tap", "opt+o");
        await ctx.waitFor(
          "the changes view",
          async () => (await ctx.captured("transform://changes", since)).length,
          10_000,
        );
        await sleep(800);
        expect(ctx.clipboard() === "transform sentinel", `clipboard: ${ctx.clipboard()}`);
        await ctx.invoke("transform_action", { action: "undo" });
        await ctx.waitFor("the original text", async () => ctx.text() === RAMBLING, 10_000);
      } finally {
        await transforms(ctx, false);
      }
    },
  },
  {
    ids: ["TRF-02"],
    name: "Opt+2 turns the selection into a structured prompt",
    async run(ctx) {
      await withOllama(ctx);
      await transforms(ctx, true);
      try {
        const rough =
          "write a script that renames my photos by the date they were taken and skips duplicates";
        selectAll(ctx, rough);
        ctx.helper("tap", "opt+2");
        await ctx.waitFor("the prompt", async () => ctx.text() !== rough, 90_000);
        soft(ctx.text().length > rough.length, `prompt: ${ctx.text()}`);
      } finally {
        await transforms(ctx, false);
      }
    },
  },
  {
    ids: ["TRF-05"],
    name: "an auto-applied transform that fails leaves the dictation as it was",
    async run(ctx) {
      await transforms(ctx, true);
      await ctx.settings({
        transforms: { autoApply: "polish" },
        ai: { provider: "ollama", cleanup: "none", ollamaUrl: "http://127.0.0.1:9" },
      });
      try {
        const since = await ctx.cursor();
        const text = await ctx.dictate("hello", { hold: 4.5 });
        expect(/Hello from Shrimp/.test(text), `inserted ${JSON.stringify(text)}`);
        await ctx.waitNotice(since, /didn't run/);
      } finally {
        await ctx.settings({
          transforms: { autoApply: null, enabled: false },
          ai: { ollamaUrl: "http://localhost:11434" },
        });
      }
    },
  },
];
