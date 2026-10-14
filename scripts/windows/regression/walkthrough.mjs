// The Windows regression walkthrough (AUD-02): the macOS cases
// (scripts/macos/regression/suites) against Shrimp Dev on Windows, or with
// `latency` the key-release-to-insert measurement (AUD-03).
//
// Run it through run.ps1, which builds the helper and the target app, starts
// the guard and puts everything back afterwards. Arguments (all optional):
//   --work <dir>      where run.ps1 put the helper, the target and the fixtures
//   --only <ids>      comma-separated PARITY ids or suite names to run
//   --report <file>   also write the results as JSON
//   --extra <file>    also run the cases a module exports as `cases` (one-off
//                     checks with apps the repository doesn't ship)
//   latency [--runs <n>] [--model <ollama model>]

import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { SENTENCE, measureLatency, option } from "../../macos/regression/latency.mjs";
import { McpClient } from "../../macos/regression/mcp-client.mjs";
import {
  COPIED,
  SPOKEN,
  SUITES,
  ollamaRunning,
  options,
  runWalkthrough,
  silentWav,
} from "../../macos/regression/runner.mjs";
import { Aborted, WinCtx } from "./context.mjs";
import { windows } from "./suites/windows.mjs";

/** The suites in order: the Windows cases before the relaunch suite, which ends with a reset app. */
async function suites() {
  const { relaunch, ...shared } = SUITES;
  const extra = option("--extra", "");
  const cases = extra ? (await import(pathToFileURL(extra).href)).cases : [];
  return { ...shared, windows, extra: cases, relaunch };
}

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "../../..");
const repoFixtures = join(repo, "src-tauri/tests/fixtures/audio");

function speak(text, path, maxSeconds = 0) {
  return execFileSync(
    "powershell",
    [
      "-NoProfile",
      "-ExecutionPolicy",
      "Bypass",
      "-File",
      join(here, "speak.ps1"),
      "-Text",
      text,
      "-Path",
      path,
      "-MaxSeconds",
      String(maxSeconds),
    ],
    { encoding: "utf8" },
  ).trim();
}

function prepareFixtures(dir) {
  mkdirSync(dir, { recursive: true });
  for (const [name, file] of Object.entries(COPIED)) {
    const target = join(dir, `${name}.wav`);
    if (!existsSync(target)) copyFileSync(join(repoFixtures, file), target);
  }
  for (const [name, text] of Object.entries(SPOKEN)) {
    const target = join(dir, `${name}.wav`);
    if (!existsSync(target)) speak(text, target);
  }
  const silence = join(dir, "silence6.wav");
  if (!existsSync(silence)) writeFileSync(silence, silentWav(6));
  const five = join(dir, "five.wav");
  if (!existsSync(five)) console.log(`latency fixture: ${speak(SENTENCE, five, 4.98)} s`);
}

/** The app's pipe, once it is up. */
async function connect() {
  const deadline = Date.now() + 60_000;
  for (;;) {
    try {
      return await McpClient.connect();
    } catch (error) {
      if (Date.now() > deadline) throw error;
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }
}

async function main() {
  const { work, only, report } = options();
  const fixtures = join(work, "fixtures");
  prepareFixtures(fixtures);
  const ctx = new WinCtx({
    app: await connect(),
    helper: join(work, "shrimp-helper.exe"),
    fixtures,
    workDir: work,
    ollama: ollamaRunning(),
    exe: process.env.SHRIMP_DEV_EXE,
    devApp: join(repo, "scripts/windows/dev-app.ps1"),
    abortFile: process.env.SHRIMP_REGRESSION_ABORT,
  });
  await ctx.waitForModel();
  if (process.argv.includes("latency")) {
    try {
      await measureLatency(ctx, {
        runs: Number(option("--runs", "10")),
        model: option("--model", ""),
      });
    } catch (error) {
      if (!(error instanceof Aborted)) throw error;
      console.log(`stopped early: ${error.message}`);
      process.exitCode = 1;
    }
    return;
  }
  // Mute while dictating is on by default on Windows (WIN-08); off here, so no
  // app the person left playing is muted. run.ps1 puts the settings back.
  await ctx.settings({ media: { mute: false } });
  process.exitCode = await runWalkthrough(ctx, {
    only,
    report,
    suites: await suites(),
    stopped: () => ctx.stopped(),
  });
}

await main();
