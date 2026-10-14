import { emit } from "@tauri-apps/api/event";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { EVENTS, type Settings, type SettingsPatch } from "../lib/ipc";
import { StylePage } from "./StylePage";
import { mockBackend, SETTINGS } from "./testBackend";

const APPS = [
  { id: "com.apple.Terminal", name: "Terminal" },
  { id: "com.tinyspeck.slackmacgap", name: "Slack" },
];

function backend(initial: Settings = SETTINGS) {
  let settings = initial;
  return mockBackend({
    get_settings: () => settings,
    get_format_apps: () => APPS,
    update_settings: (args) => {
      const patch = args.patch as SettingsPatch;
      settings = {
        ...settings,
        ai: { ...settings.ai, ...patch.ai },
        styles: { ...settings.styles, ...patch.styles },
        appFormats: patch.appFormats ?? settings.appFormats,
      };
      return settings;
    },
  });
}

describe("StylePage", () => {
  it("shows the current cleanup level and saves a new one at once", async () => {
    const { argsOf } = backend();
    render(<StylePage />);

    const group = await screen.findByRole("radiogroup", { name: "Auto Cleanup" });
    expect(within(group).getByRole("radio", { name: "Light" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    expect(within(group).getByText("Removes filler words and fixes grammar.")).toBeInTheDocument();

    fireEvent.click(within(group).getByRole("radio", { name: "Medium" }));
    await waitFor(() =>
      expect(within(group).getByRole("radio", { name: "Medium" })).toHaveAttribute(
        "aria-checked",
        "true",
      ),
    );
    expect(argsOf("update_settings")).toEqual([{ patch: { ai: { cleanup: "medium" } } }]);
  });

  it("moves the cleanup level with the arrow keys", async () => {
    const { argsOf } = backend();
    render(<StylePage />);
    const light = await screen.findByRole("radio", { name: "Light" });
    fireEvent.keyDown(light, { key: "ArrowLeft" });
    await waitFor(() =>
      expect(argsOf("update_settings")).toEqual([{ patch: { ai: { cleanup: "none" } } }]),
    );
  });

  it("offers the styles of each category in tabs", async () => {
    backend();
    render(<StylePage />);

    const tabs = await screen.findAllByRole("tab");
    expect(tabs.map((tab) => tab.textContent)).toEqual([
      "Personal messages",
      "Work messages",
      "Email",
      "Other",
    ]);
    expect(tabs[0]).toHaveAttribute("aria-selected", "true");
    expect(
      screen.getByText("Messages, WhatsApp, Discord, Signal, Telegram, Instagram"),
    ).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "very casual" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    expect(screen.queryByRole("radio", { name: "Excited!" })).not.toBeInTheDocument();
    expect(
      screen.getByText("hey are you free for lunch tomorrow? let's do 12 if that works"),
    ).toBeInTheDocument();
    expect(screen.getByText("No caps + less punctuation.")).toBeInTheDocument();

    fireEvent.keyDown(tabs[0] as HTMLElement, { key: "ArrowRight" });
    expect(screen.getByRole("tab", { name: "Work messages" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByRole("radio", { name: "Excited!" })).toBeInTheDocument();
    expect(screen.getByText("More exclamations.")).toBeInTheDocument();
    expect(screen.getByText("Slack, Microsoft Teams, LinkedIn")).toBeInTheDocument();
  });

  it("moves to the next category and hides Next on the last", async () => {
    backend();
    render(<StylePage />);
    await screen.findAllByRole("tab");
    for (const name of ["Work messages", "Email", "Other"]) {
      fireEvent.click(screen.getByRole("button", { name: "Next" }));
      expect(screen.getByRole("tab", { name })).toHaveAttribute("aria-selected", "true");
    }
    expect(screen.queryByRole("button", { name: "Next" })).not.toBeInTheDocument();
    expect(
      screen.getByText("Everything else, including AI assistants and terminals."),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Styles apply to new dictations. AI assistants such as ChatGPT and Claude get no style formatting.",
      ),
    ).toBeInTheDocument();
  });

  it("saves a style when its card is clicked", async () => {
    const { argsOf } = backend();
    render(<StylePage />);
    await screen.findAllByRole("tab");
    fireEvent.click(screen.getByRole("tab", { name: "Work messages" }));
    fireEvent.click(screen.getByRole("radio", { name: "Excited!" }));

    await waitFor(() =>
      expect(screen.getByRole("radio", { name: "Excited!" })).toHaveAttribute(
        "aria-checked",
        "true",
      ),
    );
    expect(argsOf("update_settings")).toEqual([{ patch: { styles: { work: "excited" } } }]);
  });

  it("follows settings changed elsewhere", async () => {
    backend();
    render(<StylePage />);
    const group = await screen.findByRole("radiogroup", { name: "Auto Cleanup" });
    await act(async () => {
      await emit(EVENTS.settings, { ...SETTINGS, ai: { ...SETTINGS.ai, cleanup: "none" } });
    });
    expect(within(group).getByRole("radio", { name: "None" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
  });

  it("adds an app with a format, changes it and removes it", async () => {
    const { argsOf } = backend();
    render(<StylePage />);
    const section = await screen.findByRole("region", { name: "Formatting per app" });
    const app = await within(section).findByRole("combobox", { name: "App" });
    expect(
      within(app)
        .getAllByRole("option")
        .map((option) => option.textContent),
    ).toEqual(["Terminal", "Slack"]);
    fireEvent.change(within(section).getByRole("combobox", { name: "Format" }), {
      target: { value: "lowercase" },
    });
    fireEvent.click(within(section).getByRole("button", { name: "Add app" }));
    await waitFor(() =>
      expect(argsOf("update_settings")).toEqual([
        {
          patch: {
            appFormats: [{ appId: "com.apple.Terminal", appName: "Terminal", format: "lowercase" }],
          },
        },
      ]),
    );

    const format = await within(section).findByRole("combobox", { name: "Format for Terminal" });
    expect(format).toHaveValue("lowercase");
    expect(within(section).getByRole("combobox", { name: "App" })).toHaveValue(
      "com.tinyspeck.slackmacgap",
    );
    fireEvent.change(format, { target: { value: "properNouns" } });
    await waitFor(() =>
      expect(argsOf("update_settings")[1]).toEqual({
        patch: {
          appFormats: [{ appId: "com.apple.Terminal", appName: "Terminal", format: "properNouns" }],
        },
      }),
    );
    expect(
      within(format)
        .getAllByRole("option")
        .map((option) => option.textContent),
    ).toEqual([
      "Automatic formatting",
      "Unformatted transcription",
      "Proper-noun capitals only",
      "All lowercase",
    ]);

    fireEvent.click(within(section).getByRole("button", { name: "Remove Terminal" }));
    await waitFor(() =>
      expect(argsOf("update_settings")[2]).toEqual({ patch: { appFormats: [] } }),
    );
  });

  it("offers the app in front when the window opens again", async () => {
    let apps = [{ id: "com.apple.Terminal", name: "Terminal" }];
    mockBackend({ get_settings: () => SETTINGS, get_format_apps: () => apps });
    render(<StylePage />);
    const app = await screen.findByRole("combobox", { name: "App" });
    apps = [{ id: "com.apple.Notes", name: "Notes" }, ...apps];
    await act(async () => {
      await emit(EVENTS.mainVisible, true);
    });
    await waitFor(() => expect(app).toHaveValue("com.apple.Notes"));
  });

  it("says where apps come from before any dictation", async () => {
    mockBackend({ get_settings: () => SETTINGS, get_format_apps: () => [] });
    render(<StylePage />);
    expect(await screen.findByText("Apps you dictate into appear here.")).toBeInTheDocument();
  });

  it("shows a rejected save", async () => {
    mockBackend({
      get_settings: () => SETTINGS,
      get_format_apps: () => [],
      update_settings: () => {
        throw { code: "invalid", message: "Could not save settings" };
      },
    });
    render(<StylePage />);
    fireEvent.click(await screen.findByRole("radio", { name: "Medium" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Could not save settings");
  });
});
