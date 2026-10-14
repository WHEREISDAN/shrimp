// AUD-03: key-release-to-insert latency of a 5-second dictation in the signed
// dev app, with AI edits off (cleanup None) and on (cleanup Light through the
// local Ollama). Run it through `run.sh latency`, which prepares the app.
//
// Arguments: --work <dir> (from run.sh), --runs <n> per mode (default 10),
// --model <ollama model> (default: Shrimp's choice among the installed ones).

import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";

import { Ctx, TARGET, sleep } from "./context.mjs";
import { McpClient } from "./mcp-client.mjs";

/** 4.96 s with macOS's default voice, so a 5 s hold keeps every word. */
const SENTENCE =
  "Please send the quarterly report to the whole team before Friday afternoon, and copy me.";
const HOLD_SECONDS = 5;

function option(name, fallback) {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : fallback;
}

function percentile(values, p) {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.min(sorted.length - 1, Math.ceil((p / 100) * sorted.length) - 1)];
}

/** One measured dictation: the helper holds fn and times the field's change. */
async function measure(ctx) {
  ctx.setText("");
  ctx.focusTarget();
  const since = await ctx.cursor();
  const result = ctx.helper("measure", "fn", HOLD_SECONDS, 30, TARGET);
  await ctx.waitIdle();
  const logs = await ctx.logs(since);
  const whisper = logs.map((line) => /transcribed .* in (\d+) ms/.exec(line)?.[1]).find(Boolean);
  return { ms: result.ms, whisperMs: whisper ? Number(whisper) : null, text: result.value };
}

async function mode(ctx, name, patch, runs) {
  await ctx.settings(patch);
  await measure(ctx);
  const samples = [];
  for (let run = 0; run < runs; run++) {
    samples.push(await measure(ctx));
    await sleep(1500);
  }
  const ms = samples.map((sample) => sample.ms);
  const whisper = samples.map((sample) => sample.whisperMs).filter((value) => value !== null);
  return {
    name,
    runs,
    median: percentile(ms, 50),
    p90: percentile(ms, 90),
    min: Math.min(...ms),
    max: Math.max(...ms),
    whisperMedian: whisper.length ? percentile(whisper, 50) : null,
    example: samples.at(-1).text,
  };
}

async function main() {
  const work = option("--work", join(process.env.TMPDIR ?? "/tmp", "shrimp-regression"));
  const runs = Number(option("--runs", "10"));
  const model = option("--model", "");
  const fixtures = join(work, "fixtures");
  const wav = join(fixtures, "five.wav");
  if (!existsSync(wav)) {
    execFileSync("mkdir", ["-p", fixtures]);
    execFileSync("say", ["-o", wav, "--data-format=LEI16@16000", SENTENCE]);
  }
  const ctx = new Ctx({
    app: await McpClient.connect(),
    helper: join(work, "shrimp-helper"),
    fixtures,
    workDir: work,
    ollama: true,
  });
  await ctx.source("five");
  const results = [
    await mode(ctx, "AI edits off (cleanup None)", { ai: { cleanup: "none" } }, runs),
    await mode(
      ctx,
      `AI edits on (Light, Ollama ${model || "auto"})`,
      {
        ai: {
          cleanup: "light",
          provider: "ollama",
          ollamaModel: model,
          ollamaUrl: "http://localhost:11434",
        },
      },
      runs,
    ),
  ];
  ctx.app.close();
  console.log("| Mode | Runs | Median | p90 | Min | Max | Whisper (median) |");
  console.log("| ---- | ---- | ------ | --- | --- | --- | ---------------- |");
  for (const result of results) {
    console.log(
      `| ${result.name} | ${result.runs} | ${result.median} ms | ${result.p90} ms | ${result.min} ms | ${result.max} ms | ${result.whisperMedian ?? "?"} ms |`,
    );
  }
  for (const result of results) console.log(`${result.name}: ${JSON.stringify(result.example)}`);
}

await main();
