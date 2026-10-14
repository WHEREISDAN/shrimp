// The macOS regression walkthrough: drives the running signed dev app
// ("Shrimp Dev") through tauri-mcp, synthetic keys and WAV fixtures, and
// prints one line per case with the PARITY items it covers.
//
// Run it through run.sh, which builds the helpers, launches the app and puts
// everything back afterwards. Arguments (all optional):
//   --work <dir>      where run.sh put the helper and the fixtures
//   --only <ids>      comma-separated PARITY ids or suite names to run
//   --report <file>   also write the results as JSON

import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { Ctx, Skip, Soft } from "./context.mjs";
import { McpClient } from "./mcp-client.mjs";
import { app } from "./suites/app.mjs";
import { commands } from "./suites/commands.mjs";
import { core } from "./suites/core.mjs";
import { editing } from "./suites/editing.mjs";
import { library } from "./suites/library.mjs";
import { relaunch } from "./suites/relaunch.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "../../..");
const repoFixtures = join(repo, "src-tauri/tests/fixtures/audio");

/** Spoken fixtures made with `say`; the rest come from the repository. */
const SPOKEN = {
  lines: "First line. New line. Second line. New paragraph. Third line.",
  "press-enter": "Ship it now, press enter.",
  filler: "Um, so, let's do coffee at two. Actually, three.",
  instruction: "Make this more formal.",
  question: "What is the capital of Australia?",
  tomorrow: "Tomorrow at noon.",
  names: "Please forward this to Catherine and Sean tomorrow.",
  favourite: "My favorite color is gray, and the center is organized.",
};
const COPIED = {
  hello: "hello-shrimp.wav",
  padded: "hello-shrimp-padded.wav",
  silence: "silence.wav",
  french: "french.wav",
  whispered: "whispered.wav",
  nguyen: "ask-nguyen.wav",
};

const SUITES = { core, editing, library, commands, app, relaunch };

function options() {
  const args = process.argv.slice(2);
  const value = (name) => {
    const index = args.indexOf(name);
    return index >= 0 ? args[index + 1] : undefined;
  };
  return {
    work: value("--work") ?? join(process.env.TMPDIR ?? "/tmp", "shrimp-regression"),
    only: value("--only")
      ?.split(",")
      .map((id) => id.trim()),
    report: value("--report"),
  };
}

function prepareFixtures(dir) {
  mkdirSync(dir, { recursive: true });
  for (const [name, file] of Object.entries(COPIED)) {
    const target = join(dir, `${name}.wav`);
    if (!existsSync(target)) copyFileSync(join(repoFixtures, file), target);
  }
  for (const [name, text] of Object.entries(SPOKEN)) {
    const target = join(dir, `${name}.wav`);
    if (!existsSync(target)) execFileSync("say", ["-o", target, "--data-format=LEI16@16000", text]);
  }
  const silence = join(dir, "silence6.wav");
  if (!existsSync(silence)) writeFileSync(silence, silentWav(6));
}

/** `seconds` of 16 kHz mono 16-bit digital silence (HIS-05 needs at least 5 s). */
function silentWav(seconds) {
  const samples = seconds * 16_000;
  const header = Buffer.alloc(44);
  header.write("RIFF", 0);
  header.writeUInt32LE(36 + samples * 2, 4);
  header.write("WAVEfmt ", 8);
  header.writeUInt32LE(16, 16);
  header.writeUInt16LE(1, 20);
  header.writeUInt16LE(1, 22);
  header.writeUInt32LE(16_000, 24);
  header.writeUInt32LE(32_000, 28);
  header.writeUInt16LE(2, 32);
  header.writeUInt16LE(16, 34);
  header.write("data", 36);
  header.writeUInt32LE(samples * 2, 40);
  return Buffer.concat([header, Buffer.alloc(samples * 2)]);
}

function ollamaRunning() {
  try {
    execFileSync("curl", ["-sf", "-m", "2", "http://localhost:11434/api/tags"], {
      stdio: "ignore",
    });
    return true;
  } catch {
    return false;
  }
}

function selected(only) {
  const cases = Object.entries(SUITES).flatMap(([suite, list]) =>
    list.map((item) => ({ ...item, suite })),
  );
  if (!only) return cases;
  return cases.filter(
    (item) => only.includes(item.suite) || item.ids.some((id) => only.includes(id)),
  );
}

async function main() {
  const { work, only, report } = options();
  const fixtures = join(work, "fixtures");
  prepareFixtures(fixtures);
  const client = await McpClient.connect();
  const ctx = new Ctx({
    app: client,
    helper: join(work, "shrimp-helper"),
    fixtures,
    workDir: work,
    ollama: ollamaRunning(),
    bundle: join(repo, "src-tauri/target/debug/bundle/macos/Shrimp Dev.app"),
  });
  const results = [];
  for (const item of selected(only)) {
    const started = Date.now();
    let status = "pass";
    let detail = "";
    try {
      await item.run(ctx);
    } catch (error) {
      status = error instanceof Skip ? "skip" : error instanceof Soft ? "warn" : "fail";
      detail = error.message;
      await ctx.recover().catch(() => {});
    }
    const seconds = ((Date.now() - started) / 1000).toFixed(1);
    results.push({ suite: item.suite, ids: item.ids, name: item.name, status, detail, seconds });
    console.log(
      `${status.toUpperCase().padEnd(4)} ${item.ids.join(", ").padEnd(28)} ${item.name} (${seconds} s)${detail ? `\n     ${detail}` : ""}`,
    );
  }
  ctx.app.close();
  const count = (status) => results.filter((result) => result.status === status).length;
  console.log(
    `\n${count("pass")} passed, ${count("fail")} failed, ${count("warn")} model warnings, ${count("skip")} skipped`,
  );
  for (const note of ctx.notes) console.log(`note: ${note}`);
  if (report) writeFileSync(report, JSON.stringify({ results, notes: ctx.notes }, null, 2));
  process.exitCode = count("fail") > 0 ? 1 : 0;
}

await main();
