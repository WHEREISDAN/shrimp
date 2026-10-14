// Cases that restart the app: a crash during a dictation (HIS-06, APP-09),
// the no-Fn defaults (CORE-10) and a launch from the login item (APP-06).
// The last case relaunches normally, so it runs last.

import { execFileSync } from "node:child_process";
import { statSync } from "node:fs";
import { join } from "node:path";

import { expect, sleep } from "../context.mjs";
import { McpClient } from "../mcp-client.mjs";

const SOCKET = process.env.TAURI_MCP_IPC_PATH ?? "/tmp/tauri-mcp.sock";
const DEV_APP = join(import.meta.dirname, "../../dev-app.sh");

/** Relaunches Shrimp Dev with `env` and reconnects once its new socket answers. */
async function restart(ctx, env = {}) {
  const before = tokenTime();
  ctx.app.close();
  const args = [
    "open",
    ...Object.entries(env).flatMap(([name, value]) => ["--env", `${name}=${value}`]),
  ];
  execFileSync(DEV_APP, args, { stdio: "ignore" });
  await ctx.waitFor("the new socket", async () => tokenTime() > before, 30_000);
  await sleep(1500);
  ctx.app = await McpClient.connect(SOCKET);
  await ctx.waitFor(
    "the app to answer",
    async () => (await ctx.invoke("get_dictation_state").catch(() => null)) !== null,
    30_000,
  );
  await sleep(1500);
}

function tokenTime() {
  try {
    return statSync(`${SOCKET}.token`).mtimeMs;
  } catch {
    return 0;
  }
}

export const relaunch = [
  {
    ids: ["HIS-06", "APP-09", "CORE-10"],
    name: "a crash mid-dictation: the crash screen, the recovered recording; no-Fn defaults",
    async run(ctx) {
      await ctx.source("padded");
      ctx.focusTarget();
      await ctx.invoke("flow_bar_control", { control: "start" });
      await sleep(3000);
      execFileSync("kill", ["-9", String(ctx.appPid())]);
      await sleep(1000);
      await restart(ctx, { SHRIMP_FN_KEY: "0" });
      const health = await ctx.invoke("get_app_health");
      expect(health.crash, `no crash screen: ${JSON.stringify(health)}`);
      await ctx.invoke("dismiss_crash");
      await ctx.waitNotice(0, /saved a dictation that was interrupted/);
      const { entries } = await ctx.invoke("list_history", { query: "" });
      const interrupted = entries.find((entry) => entry.status === "interrupted");
      expect(interrupted, `no interrupted entry: ${JSON.stringify(entries[0])}`);
      const since = await ctx.cursor();
      await ctx.invoke("retry_history", { id: interrupted.id });
      await ctx
        .waitFor(
          "the recovered text",
          async () => {
            const list = await ctx.invoke("list_history", { query: "" });
            return list.entries
              .find((entry) => entry.id === interrupted.id)
              ?.finalText?.includes("Hello");
          },
          40_000,
        )
        .catch(async (error) => {
          const list = await ctx.invoke("list_history", { query: "" });
          const entry = list.entries.find((item) => item.id === interrupted.id);
          throw new Error(
            `${error.message}: ${JSON.stringify(entry)} ${(await ctx.logs(since)).slice(-8).join(" | ")}`,
          );
        });
      const push = (await ctx.invoke("get_shortcuts")).rows.find(
        (row) => row.action === "pushToTalk",
      );
      expect(push.shortcuts[0] === "⌃⌥", `no-Fn push to talk: ${push.shortcuts}`);
    },
  },
  {
    ids: ["APP-06"],
    name: "a launch from the login item keeps the main window hidden",
    async run(ctx) {
      await restart(ctx, { SHRIMP_LAUNCHED_AT_LOGIN: "1" });
      expect(!(await ctx.windowVisible("main")), "the main window opened");
      expect(await ctx.windowVisible("flow-bar"), "no Flow Bar");
    },
  },
  {
    ids: ["APP-08"],
    name: "Reset & restart starts over with the defaults and keeps the models",
    async run(ctx) {
      const before = tokenTime();
      ctx.app.close();
      const client = await McpClient.connect(SOCKET);
      await client.invoke("reset_app").catch(() => {});
      client.close();
      await ctx.waitFor("the restarted app", async () => tokenTime() > before, 60_000);
      await sleep(2500);
      ctx.app = await McpClient.connect(SOCKET);
      const settings = await ctx.invoke("get_settings");
      expect(!settings.onboarding.completed, `onboarding: ${JSON.stringify(settings.onboarding)}`);
      expect(
        (await ctx.invoke("list_history", { query: "" })).entries.length === 0,
        "history survived",
      );
      const active = (await ctx.invoke("get_models")).find((model) => model.isDefault);
      expect(active.state !== "missing", `the default model is ${active.state}`);
      ctx.note("APP-08 reset the dev app's data; run.sh puts the saved data back afterwards");
    },
  },
  {
    ids: ["APP-01"],
    name: "a normal launch opens the main window",
    async run(ctx) {
      await restart(ctx);
      await ctx.waitFor("the main window", async () => ctx.windowVisible("main"), 10_000);
      await ctx.source("hello");
      ctx.focusTarget();
    },
  },
];
