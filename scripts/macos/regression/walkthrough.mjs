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

import { Ctx } from "./context.mjs";
import { McpClient } from "./mcp-client.mjs";
import { COPIED, SPOKEN, ollamaRunning, options, runWalkthrough, silentWav } from "./runner.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "../../..");
const repoFixtures = join(repo, "src-tauri/tests/fixtures/audio");

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
  process.exitCode = await runWalkthrough(ctx, { only, report });
}

await main();
