// AI edits, styles, context awareness and coding (phases 2–3). Cases that
// need a model use the local Ollama; the rest run the pipeline without one.

import { TARGET, expect, expectIncludes, soft } from "../context.mjs";

const CURSOR = "com.todesktop.230313mzl4w4u92";

/** Ollama with its first installed model, loaded before the first check. */
export async function withOllama(ctx, cleanup = "light") {
  ctx.requireOllama();
  await ctx.settings({
    ai: { provider: "ollama", cleanup, ollamaModel: "", ollamaUrl: "http://localhost:11434" },
  });
  if (!ctx.ollamaWarm) {
    await ctx.invoke("test_ai_provider").catch(() => {});
    ctx.ollamaWarm = true;
  }
}

function edit(ctx, text, appId = TARGET) {
  return ctx.invoke("dev_edit_text", { text, appId });
}

async function asCategory(ctx, category, work) {
  await ctx.invoke("dev_set_app_category", { appId: TARGET, category });
  try {
    return await work();
  } finally {
    await ctx.invoke("dev_set_app_category", { appId: TARGET, category: null });
  }
}

export const editing = [
  {
    ids: ["FMT-01", "FMT-02", "FMT-03"],
    name: "None keeps the transcript; Light removes fillers and keeps the correction",
    async run(ctx) {
      const spoken = "Um, so, let's do coffee at two. Actually, three.";
      await ctx.settings({ ai: { cleanup: "none" } });
      const none = await edit(ctx, spoken);
      expect(none.plain === spoken, `None changed it: ${JSON.stringify(none.plain)}`);
      await withOllama(ctx);
      const light = await edit(ctx, spoken);
      expect(light.formatted, `Light fell back: ${light.notice}`);
      soft(!/\bum\b/i.test(light.plain), `filler kept: ${light.plain}`);
      soft(
        /three|3/i.test(light.plain) && !/\btwo\b|\b2\b/i.test(light.plain),
        `backtrack: ${light.plain}`,
      );
    },
  },
  {
    ids: ["FMT-04", "FMT-05"],
    name: "spoken lists and punctuation words",
    async run(ctx) {
      await withOllama(ctx);
      const list = await edit(
        ctx,
        "My to-do list. One, buy milk. Two, call the bank. Three, book the flights.",
      );
      expect(list.formatted, `fell back: ${list.notice}`);
      soft(
        /<li>|^\s*1[.)]/m.test(`${list.html ?? ""}\n${list.plain}`),
        `no list: ${JSON.stringify(list)}`,
      );
      const marks = await edit(ctx, "Are you coming tonight question mark");
      soft(
        /\?$/.test(marks.plain.trim()) && !/question mark/i.test(marks.plain),
        `punctuation: ${marks.plain}`,
      );
    },
  },
  {
    ids: ["FMT-07", "STY-02"],
    name: "styles change only casing and punctuation",
    async run(ctx) {
      await withOllama(ctx);
      await ctx.settings({ styles: { personal: "veryCasual", work: "excited" } });
      try {
        const casual = await asCategory(ctx, "personal", () =>
          edit(ctx, "Let's do coffee at three."),
        );
        expect(!/\.$/.test(casual.plain.trim()), `very casual kept the period: ${casual.plain}`);
        expect(
          casual.plain.trim()[0] === casual.plain.trim()[0].toLowerCase(),
          `very casual capital: ${casual.plain}`,
        );
        const excited = await asCategory(ctx, "work", () =>
          edit(ctx, "Great job on the launch everyone."),
        );
        const words = (text) =>
          text
            .toLowerCase()
            .replace(/[^a-z ]/g, "")
            .split(/\s+/)
            .join(" ");
        expect(
          !/\.$/.test(excited.plain.trim()),
          `a work message kept its period: ${excited.plain}`,
        );
        soft(
          words(excited.plain) === "great job on the launch everyone",
          `wording changed: ${excited.plain}`,
        );
        soft(/!$/.test(excited.plain.trim()), `excited without “!”: ${excited.plain}`);
      } finally {
        await ctx.settings({ styles: { personal: "formal", work: "formal" } });
      }
    },
  },
  {
    ids: ["FMT-08", "STY-01"],
    name: "email apps get a greeting, body and sign-off on their own lines",
    async run(ctx) {
      await withOllama(ctx);
      const email = await asCategory(ctx, "email", () =>
        edit(
          ctx,
          "Hi Sam, thanks for sending the contract over. I will review it tomorrow. Best, Dan.",
        ),
      );
      expect(email.formatted, `fell back: ${email.notice}`);
      soft(
        /Sam,\s*\n/.test(email.plain) && /\n\s*Best,?/.test(email.plain),
        `email: ${JSON.stringify(email.plain)}`,
      );
    },
  },
  {
    ids: ["FMT-09"],
    name: "AI chat apps get no style formatting",
    async run(ctx) {
      await withOllama(ctx);
      const prompt = await asCategory(ctx, "ai", () =>
        edit(ctx, "write me a list one apples two pears three plums"),
      );
      expect(prompt.formatted, `fell back: ${prompt.notice}`);
      soft(
        !prompt.html && !/^\s*1[.)]/m.test(prompt.plain),
        `the model made a list for an AI app: ${JSON.stringify(prompt.plain)}`,
      );
    },
  },
  {
    ids: ["FMT-11", "CTX-01"],
    name: "mid-sentence insertions continue the sentence",
    async run(ctx) {
      await ctx.settings({ ai: { cleanup: "none" } });
      ctx.setText("We should talk about it");
      const context = await ctx.invoke("dev_read_context");
      expect(
        context.app?.includes(TARGET) && context.before.endsWith("talk about it"),
        `context: ${JSON.stringify(context)}`,
      );
      const text = await ctx.dictate("tomorrow", { hold: 3, keepText: true });
      expect(/about it tomorrow at noon\.?$/.test(text), `inserted: ${JSON.stringify(text)}`);
    },
  },
  {
    ids: ["CTX-02"],
    name: "a name on screen is spelled as shown",
    async run(ctx) {
      await withOllama(ctx);
      ctx.setText("Hi Kathryn and Shawn,\n");
      const text = await ctx.dictate("names", { hold: 5, keepText: true });
      expectIncludes(text, "Kathryn");
      expectIncludes(text, "Shawn");
    },
  },
  {
    ids: ["CTX-03"],
    name: "password fields are never read",
    async run(ctx) {
      ctx.focusTarget("secure");
      const context = await ctx.invoke("dev_read_context");
      ctx.focusTarget("text");
      expect(
        !context.readable && /secure/i.test(context.fieldKind),
        `secure field: ${JSON.stringify(context)}`,
      );
    },
  },
  {
    ids: ["DEV-03"],
    name: "case commands and CLI words in developer apps",
    async run(ctx) {
      await ctx.settings({ ai: { cleanup: "none" } });
      const camel = await edit(ctx, "camel case user profile", CURSOR);
      expectIncludes(camel.plain, "userProfile");
      const snake = await edit(ctx, "snake case max retry count", CURSOR);
      expectIncludes(snake.plain, "max_retry_count");
    },
  },
];
