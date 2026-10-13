import { emit } from "@tauri-apps/api/event";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { EVENTS, type Settings, type SettingsPatch } from "../lib/ipc";
import { StylePage } from "./StylePage";
import { mockBackend, SETTINGS } from "./testBackend";

function backend(initial: Settings = SETTINGS) {
  let settings = initial;
  return mockBackend({
    get_settings: () => settings,
    update_settings: (args) => {
      const patch = args.patch as SettingsPatch;
      settings = {
        ...settings,
        ai: { ...settings.ai, ...patch.ai },
        styles: { ...settings.styles, ...patch.styles },
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

  it("shows a rejected save", async () => {
    mockBackend({
      get_settings: () => SETTINGS,
      update_settings: () => {
        throw { code: "invalid", message: "Could not save settings" };
      },
    });
    render(<StylePage />);
    fireEvent.click(await screen.findByRole("radio", { name: "Medium" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Could not save settings");
  });
});
