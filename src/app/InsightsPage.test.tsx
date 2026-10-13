import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { VoiceProfileStatus } from "../lib/ipc";
import { band } from "./ActivityHeatmap";
import { InsightsPage } from "./InsightsPage";
import { INSIGHTS, mockBackend, SETTINGS, VOICE_PROFILE } from "./testBackend";

const PROFILE: VoiceProfileStatus = {
  ...VOICE_PROFILE,
  profile: {
    archetype: "The Swift Strategist",
    description: "Quick, clear and to the point.",
    catchphrase: "let's ship it",
    mostCorrected: "um",
    activeTimes: "Tuesday mornings",
    generatedAt: Date.UTC(2026, 9, 13),
    words: 2_480,
  },
  nextAt: 7_000,
};

function backend(handlers: Record<string, (args: Record<string, unknown>) => unknown> = {}) {
  return mockBackend({
    get_insights: () => INSIGHTS,
    get_voice_profile: () => VOICE_PROFILE,
    get_settings: () => SETTINGS,
    generate_voice_profile: () => PROFILE,
    ...handlers,
  });
}

describe("InsightsPage", () => {
  it("shows speed, totals, fixes, streaks, activity and apps", async () => {
    backend();
    render(<InsightsPage />);
    expect(await screen.findByText("142 WPM")).toBeInTheDocument();
    expect(screen.getByText("2,480 words")).toBeInTheDocument();
    expect(screen.getByText("+20% this month")).toBeInTheDocument();
    expect(screen.getByText("322 words")).toBeInTheDocument();
    expect(screen.getByText("3 days")).toBeInTheDocument();
    expect(screen.getByText("5 days")).toBeInTheDocument();
    expect(screen.getByText("Top app: Slack")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: /2 active days, 640 words/ })).toBeInTheDocument();
    const apps = screen.getByRole("list");
    expect(within(apps).getAllByRole("listitem")).toHaveLength(2);
  });

  it("bands days like Wispr Flow", () => {
    expect([0, 1, 249, 250, 499, 500, 750, 5_000].map(band)).toEqual([0, 1, 1, 2, 2, 3, 4, 4]);
  });

  it("generates the voice profile in the interface language", async () => {
    const { argsOf } = backend();
    render(<InsightsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Generate profile" }));
    expect(await screen.findByText("The Swift Strategist")).toBeInTheDocument();
    expect(screen.getByText("“let's ship it”")).toBeInTheDocument();
    expect(screen.getByText("Next update in 4,520 more words")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Regenerate profile" })).toBeDisabled();
    expect(argsOf("generate_voice_profile")).toEqual([{ language: "English" }]);
  });

  it("explains when too few words are stored locally", async () => {
    backend({
      generate_voice_profile: () => {
        throw { kind: "notEnoughStored", stored: 900, needed: 2_000 };
      },
    });
    render(<InsightsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Generate profile" }));
    expect(await screen.findByText("You only have 900 words stored locally.")).toBeInTheDocument();
    expect(
      screen.getByText(
        "You will need to dictate 1,100 more words so you have enough data to build your profile.",
      ),
    ).toBeInTheDocument();
  });

  it("shows how far the first profile is", async () => {
    backend({ get_voice_profile: () => ({ ...VOICE_PROFILE, totalWords: 1_500 }) });
    render(<InsightsPage />);
    expect(await screen.findByText("Generates in 500 words")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Generate profile" })).toBeDisabled();
  });

  it("copies and downloads the share card", async () => {
    const context = new Proxy(
      { measureText: () => ({ width: 10 }) },
      { get: (target, key) => (key in target ? target[key as "measureText"] : () => undefined) },
    );
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(
      context as unknown as CanvasRenderingContext2D,
    );
    vi.spyOn(HTMLCanvasElement.prototype, "toBlob").mockImplementation((callback) =>
      callback(new Blob([new Uint8Array([0x89, 0x50, 0x4e, 0x47])], { type: "image/png" })),
    );
    const { calls } = backend({
      copy_image: () => null,
      save_image: () => "/Users/me/Downloads/My Shrimp activity 2026-10-13.png",
    });
    render(<InsightsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Share" }));
    const dialog = screen.getByRole("dialog", { name: "Share your stats" });
    const copy = within(dialog).getByRole("button", { name: "Copy image" });
    await waitFor(() => expect(copy).toBeEnabled());
    fireEvent.click(copy);
    expect(await within(dialog).findByText("Image copied")).toBeInTheDocument();
    fireEvent.click(within(dialog).getByRole("button", { name: "Download" }));
    expect(await within(dialog).findByText(/Saved to .*Downloads/)).toBeInTheDocument();
    expect(calls.map((call) => call.cmd)).toContain("save_image");
  });
});
