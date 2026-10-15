// Dictionary and snippets (phase 2), through their IPC commands and live dictations.

import { TARGET, expect, expectIncludes } from "../context.mjs";

async function rejects(promise) {
  try {
    await promise;
  } catch (error) {
    return error.message;
  }
  throw new Error("expected an error");
}

/** Deletes the dictionary entries and snippets a case added, whatever happened. */
async function cleanUp(ctx, words, triggers) {
  const entries = await ctx.invoke("list_dictionary");
  const ids = entries.filter((entry) => words.includes(entry.term)).map((entry) => entry.id);
  if (ids.length) await ctx.invoke("delete_dictionary_entries", { ids });
  const snippets = await ctx.invoke("list_snippets");
  const snippetIds = snippets
    .filter((snippet) => triggers.includes(snippet.trigger))
    .map((snippet) => snippet.id);
  if (snippetIds.length) await ctx.invoke("delete_snippets", { ids: snippetIds });
}

export const library = [
  {
    ids: ["DIC-01", "DIC-04", "DIC-05"],
    name: "dictionary words: add, reject, star, edit and delete",
    async run(ctx) {
      try {
        const added = await ctx.invoke("add_dictionary_entry", { input: { term: "Nguyen" } });
        expect(added.term === "Nguyen", `added ${JSON.stringify(added)}`);
        expectIncludes(
          await rejects(ctx.invoke("add_dictionary_entry", { input: { term: "nguyen" } })),
          "already exists",
        );
        expectIncludes(
          await rejects(ctx.invoke("add_dictionary_entry", { input: { term: "!!!" } })),
          "letter or number",
        );
        expectIncludes(
          await rejects(ctx.invoke("add_dictionary_entry", { input: { term: "  " } })),
          "empty",
        );
        await rejects(ctx.invoke("add_dictionary_entry", { input: { term: "x".repeat(61) } }));
        const pinned = await ctx.invoke("set_dictionary_pinned", { id: added.id, pinned: true });
        expect(pinned.pinned, `pin: ${JSON.stringify(pinned)}`);
        const edited = await ctx.invoke("update_dictionary_entry", {
          id: added.id,
          input: { term: "Nguyễn" },
        });
        expect(edited.term === "Nguyễn", `edit: ${JSON.stringify(edited)}`);
        const deleted = await ctx.invoke("delete_dictionary_entries", { ids: [added.id] });
        expect(deleted === 1, `deleted ${deleted}`);
      } finally {
        await cleanUp(ctx, ["Nguyen", "Nguyễn"], []);
      }
    },
  },
  {
    ids: ["DIC-02"],
    name: "a dictionary word steers Whisper",
    async run(ctx) {
      await ctx.settings({ ai: { cleanup: "none" } });
      const without = await ctx.dictate("nguyen", { hold: 4.5 });
      expect(!without.includes("Nguyen"), `already right without the dictionary: ${without}`);
      const added = await ctx.invoke("add_dictionary_entry", { input: { term: "Nguyen" } });
      try {
        await ctx.invoke("set_dictionary_pinned", { id: added.id, pinned: true });
        expectIncludes(await ctx.dictate("nguyen", { hold: 4.5 }), "Nguyen");
      } finally {
        await cleanUp(ctx, ["Nguyen"], []);
      }
    },
  },
  {
    ids: ["DIC-03"],
    name: "a correction replaces whole words in any case",
    async run(ctx) {
      await ctx.settings({ ai: { cleanup: "none" } });
      await ctx.invoke("add_dictionary_entry", {
        input: { term: "Kubernetes", misspelling: "cooper netties" },
      });
      try {
        const edited = await ctx.invoke("dev_edit_text", {
          text: "We moved everything onto Cooper Netties last week.",
          appId: TARGET,
        });
        expectIncludes(edited.plain, "onto Kubernetes last week");
      } finally {
        await cleanUp(ctx, ["Kubernetes"], []);
      }
    },
  },
  {
    ids: ["DIC-06"],
    name: "a CSV import previews duplicates and marks rows imported",
    async run(ctx) {
      await ctx.invoke("add_dictionary_entry", { input: { term: "Shrimpish" } });
      try {
        const csv = "Shrimpish\nTauri\nwisper,Wispr\na,b,c\n!!!\n";
        const preview = await ctx.invoke("preview_dictionary_import", { csv });
        const text = JSON.stringify(preview);
        expect(/Tauri/.test(text) && /Shrimpish/.test(text), `preview: ${text}`);
        const imported = await ctx.invoke("import_dictionary", { csv });
        expect(imported === 2, `imported ${imported}`);
        const entries = await ctx.invoke("list_dictionary");
        const tauri = entries.find((entry) => entry.term === "Tauri");
        expect(
          tauri && /import/i.test(JSON.stringify(tauri)),
          `imported row: ${JSON.stringify(tauri)}`,
        );
      } finally {
        await cleanUp(ctx, ["Shrimpish", "Tauri", "Wispr"], []);
      }
    },
  },
  {
    ids: ["SNP-01", "SNP-02", "SNP-04", "SNP-05"],
    name: "snippets: limits, matching, similar triggers and import",
    async run(ctx) {
      const triggers = ["my calendar link", "my calendar links", "signature"];
      try {
        await rejects(
          ctx.invoke("add_snippet", { input: { trigger: "t".repeat(61), expansion: "x" } }),
        );
        await rejects(
          ctx.invoke("add_snippet", { input: { trigger: "long", expansion: "x".repeat(4001) } }),
        );
        await ctx.invoke("add_snippet", {
          input: { trigger: "my calendar link", expansion: "https://cal.com/alex" },
        });
        const similar = await ctx.invoke("add_snippet", {
          input: { trigger: "my calendar links", expansion: "x" },
        });
        expect(
          /similar/i.test(JSON.stringify(similar)),
          `no similar-trigger warning: ${JSON.stringify(similar)}`,
        );
        await ctx.settings({ ai: { cleanup: "none" } });
        const inSentence = await ctx.invoke("dev_edit_text", {
          text: "Book a time with me here, my calendar link. Thanks.",
          appId: TARGET,
        });
        expectIncludes(inSentence.plain, "https://cal.com/alex");
        const json = JSON.stringify([
          { name: "signature", text: "Alex Rivera" },
          { name: "my calendar link", text: "dup" },
        ]);
        const preview = await ctx.invoke("preview_snippet_import", { json });
        expect(/signature/.test(JSON.stringify(preview)), `preview: ${JSON.stringify(preview)}`);
        expect((await ctx.invoke("import_snippets", { json })) === 1, "import count");
        const alone = await ctx.invoke("dev_edit_text", { text: "Signature.", appId: TARGET });
        expectIncludes(alone.plain, "Alex Rivera");
      } finally {
        await cleanUp(ctx, [], triggers);
      }
    },
  },
];
