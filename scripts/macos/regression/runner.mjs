// What the macOS and Windows walkthroughs share: the suites, the fixtures,
// the command line and the loop that runs the cases and prints one line per
// case with the PARITY items it covers.

import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { Skip, Soft } from "./context.mjs";
import { app } from "./suites/app.mjs";
import { audit } from "./suites/audit.mjs";
import { commands } from "./suites/commands.mjs";
import { core } from "./suites/core.mjs";
import { editing } from "./suites/editing.mjs";
import { library } from "./suites/library.mjs";
import { relaunch } from "./suites/relaunch.mjs";

export const SUITES = { core, editing, library, commands, app, audit, relaunch };

/** Fixtures spoken by the platform's text to speech; the rest come from the repository. */
export const SPOKEN = {
  lines: "First line. New line. Second line. New paragraph. Third line.",
  "press-enter": "Ship it now, press enter.",
  filler: "Um, so, let's do coffee at two. Actually, three.",
  instruction: "Make this more formal.",
  question: "What is the capital of Australia?",
  tomorrow: "Tomorrow at noon.",
  names: "Please forward this to Catherine and Sean tomorrow.",
  favourite: "My favorite color is gray, and the center is organized.",
};

export const COPIED = {
  hello: "hello-shrimp.wav",
  padded: "hello-shrimp-padded.wav",
  silence: "silence.wav",
  french: "french.wav",
  whispered: "whispered.wav",
  nguyen: "ask-nguyen.wav",
};

/**
 * The command line, all optional:
 *   --work <dir>      where run.sh or run.ps1 put the helper and the fixtures
 *   --only <ids>      comma-separated PARITY ids or suite names to run
 *   --report <file>   also write the results as JSON
 */
export function options() {
  const args = process.argv.slice(2);
  const value = (name) => {
    const index = args.indexOf(name);
    return index >= 0 ? args[index + 1] : undefined;
  };
  return {
    work: value("--work") ?? join(tmpdir(), "shrimp-regression"),
    only: value("--only")
      ?.split(",")
      .map((id) => id.trim()),
    report: value("--report"),
  };
}

/** `seconds` of 16 kHz mono 16-bit digital silence (HIS-05 needs at least 5 s). */
export function silentWav(seconds) {
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

export function ollamaRunning() {
  try {
    execFileSync("curl", ["-sf", "-m", "2", "http://localhost:11434/api/tags"], {
      stdio: "ignore",
    });
    return true;
  } catch {
    return false;
  }
}

/** The cases for this platform, narrowed to `only` (suites or PARITY ids). */
function selected(suites, only) {
  const cases = Object.entries(suites)
    .flatMap(([suite, list]) => list.map((item) => ({ ...item, suite })))
    .filter((item) => !item.platforms || item.platforms.includes(process.platform));
  if (!only) return cases;
  return cases.filter(
    (item) => only.includes(item.suite) || item.ids.some((id) => only.includes(id)),
  );
}

/**
 * Runs the selected cases of `suites` and prints the results. `stopped()` is
 * asked after every case; a reason it returns ends the run there. Returns the
 * exit status.
 */
export async function runWalkthrough(ctx, { only, report, suites = SUITES, stopped = () => null }) {
  const results = [];
  let reason = null;
  for (const item of selected(suites, only)) {
    const started = Date.now();
    const since = report ? await ctx.cursor().catch(() => undefined) : undefined;
    let status = "pass";
    let detail = "";
    let logs;
    try {
      await item.run(ctx);
    } catch (error) {
      status = error instanceof Skip ? "skip" : error instanceof Soft ? "warn" : "fail";
      detail = error.message;
      // A failure's own log lines, for the report (the app may restart later).
      if (since !== undefined) logs = (await ctx.logs(since).catch(() => [])).slice(-120);
      if (!stopped()) await ctx.recover().catch(() => {});
    }
    const seconds = ((Date.now() - started) / 1000).toFixed(1);
    const result = { suite: item.suite, ids: item.ids, name: item.name, status, detail, seconds };
    results.push(logs ? { ...result, logs } : result);
    console.log(
      `${status.toUpperCase().padEnd(4)} ${item.ids.join(", ").padEnd(28)} ${item.name} (${seconds} s)${detail ? `\n     ${detail}` : ""}`,
    );
    reason = stopped();
    if (reason) break;
  }
  if (report) {
    // The app's log since its last start, beside the report, for failures to be read later.
    const logs = await ctx.app.logs(0, { limit: 100_000 }).catch(() => null);
    const lines = (logs?.entries ?? []).map((entry) => JSON.stringify(entry));
    writeFileSync(report.replace(/\.json$/i, "") + ".app.log", lines.join("\n"));
  }
  ctx.app.close();
  const count = (status) => results.filter((result) => result.status === status).length;
  console.log(
    `\n${count("pass")} passed, ${count("fail")} failed, ${count("warn")} model warnings, ${count("skip")} skipped`,
  );
  if (reason) console.log(`stopped early: ${reason}`);
  for (const note of ctx.notes) console.log(`note: ${note}`);
  if (report) writeFileSync(report, JSON.stringify({ results, notes: ctx.notes, reason }, null, 2));
  return count("fail") > 0 || reason ? 1 : 0;
}
