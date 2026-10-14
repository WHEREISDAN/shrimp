// Cases that restart the app: a crash during a dictation (HIS-06, APP-09),
// the no-Fn defaults (CORE-10, macOS) and a launch from the login item
// (APP-06). The last case relaunches normally, so it runs last.

import { expect, expectIncludes, sleep } from "../context.mjs";

const MAC = process.platform === "darwin";

export const relaunch = [
  {
    ids: ["HIS-06", "APP-09", ...(MAC ? ["CORE-10"] : [])],
    name: "a crash mid-dictation: the crash screen, the recovered recording; no-Fn defaults",
    async run(ctx) {
      await ctx.source("lines");
      ctx.focusTarget();
      await ctx.invoke("flow_bar_control", { control: "start" });
      await sleep(4000);
      ctx.killApp();
      await sleep(1000);
      await ctx.restart(MAC ? { SHRIMP_FN_KEY: "0" } : {});
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
              ?.finalText?.includes("line");
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
      if (MAC) {
        const push = (await ctx.invoke("get_shortcuts")).rows.find(
          (row) => row.action === "pushToTalk",
        );
        expect(push.shortcuts[0] === "⌃⌥", `no-Fn push to talk: ${push.shortcuts}`);
      }
    },
  },
  {
    ids: ["APP-06"],
    name: "a launch from the login item keeps the main window hidden",
    async run(ctx) {
      await ctx.restart({ SHRIMP_LAUNCHED_AT_LOGIN: "1" });
      expect(!(await ctx.windowVisible("main")), "the main window opened");
      expect(await ctx.windowVisible("flow-bar"), "no Flow Bar");
    },
  },
  {
    ids: ["APP-11"],
    name: "outside the Applications folder Shrimp asks to be moved and no shortcut dictates",
    platforms: ["darwin"],
    async run(ctx) {
      await ctx.restart({ SHRIMP_PRETEND_OUTSIDE_APPLICATIONS: "1" });
      const health = await ctx.invoke("get_app_health");
      expect(health.misplaced, `health: ${JSON.stringify(health)}`);
      ctx.setText("");
      ctx.focusTarget();
      const since = await ctx.cursor();
      ctx.helper("hold", "fn", 2);
      await sleep(1500);
      expect(
        !(await ctx.logs(since)).some((line) => /recording a dictation/.test(line)),
        "fn dictated",
      );
      await ctx.invoke("move_to_applications").catch(() => {});
      await ctx.waitLog(since, /would copy/);
    },
  },
  {
    ids: ["LANG-07", "CORE-18"],
    name: "in Canada Shrimp offers Canadian English, then a mouse button for dictating",
    async run(ctx) {
      await ctx.restart({ SHRIMP_REGION: "CA", SHRIMP_FAKE_MOUSE_BUTTONS: "5" });
      await ctx.invoke("set_languages", { autoDetect: false, selected: ["en-US"] });
      await ctx.settings({
        ai: { cleanup: "none" },
        notices: { canadianEnglishShown: false, mouseButtonsShown: false },
      });
      let since = await ctx.cursor();
      await ctx.dictate("hello", { hold: 4.5 });
      await ctx.waitNotice(since, /Switch to Canadian English\?/);
      await ctx.invoke("run_notice_action", { action: "switchToCanadianEnglish" });
      await ctx.waitFor(
        "English (Canada)",
        async () => (await ctx.invoke("get_settings")).languages.selected.includes("en-CA"),
        5_000,
      );
      // The next dictation, with Canadian English already offered, reads the mice.
      since = await ctx.cursor();
      expectIncludes(await ctx.dictate("favourite", { hold: 5 }), "favourite colour");
      await ctx.waitNotice(since, /Your mouse has extra buttons/);
      await ctx.invoke("set_languages", { autoDetect: false, selected: ["en-US"] });
    },
  },
  {
    ids: ["APP-12"],
    name: "Restart Shrimp in the tray relaunches with the main window and no crash screen",
    async run(ctx) {
      await ctx.restart();
      const pid = ctx.appPid();
      await ctx.relaunched(async () => ctx.trayItems("Restart Shrimp"));
      expect(ctx.appPid() !== pid, "the same process is still running");
      await ctx.waitFor("the main window", async () => ctx.windowVisible("main"), 10_000);
      const health = await ctx.invoke("get_app_health");
      expect(!health.crash, `crash screen after a restart: ${JSON.stringify(health)}`);
      ctx.focusTarget();
    },
  },
  {
    ids: ["APP-08"],
    name: "Reset & restart starts over with the defaults and keeps the models",
    async run(ctx) {
      await ctx.relaunched(async () => {
        const client = await ctx.connect();
        await client.invoke("reset_app").catch(() => {});
        client.close();
      });
      const settings = await ctx.invoke("get_settings");
      expect(!settings.onboarding.completed, `onboarding: ${JSON.stringify(settings.onboarding)}`);
      expect(
        (await ctx.invoke("list_history", { query: "" })).entries.length === 0,
        "history survived",
      );
      const active = (await ctx.invoke("get_models")).find((model) => model.isDefault);
      expect(active.state !== "missing", `the default model is ${active.state}`);
      ctx.note("APP-08 reset the dev app's data; the run puts the saved data back afterwards");
    },
  },
  {
    ids: ["APP-01"],
    name: "a normal launch opens the main window",
    async run(ctx) {
      await ctx.restart();
      await ctx.waitFor("the main window", async () => ctx.windowVisible("main"), 10_000);
      await ctx.source("hello");
      ctx.focusTarget();
    },
  },
];
