import { act, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Greeting } from "./Greeting";
import { INSIGHTS, mockBackend, SHORTCUTS } from "./testBackend";

function renderGreeting(totalWords: number) {
  mockBackend({
    app_info: () => ({
      name: "Shrimp",
      version: "0.1.0",
      os: "macos",
      arch: "aarch64",
      userName: "Dan",
    }),
    get_shortcuts: () => SHORTCUTS,
    get_insights: () => ({ ...INSIGHTS, totalWords }),
  });
  render(<Greeting onOpenInsights={() => undefined} />);
}

/** Lets every pending IPC answer arrive. */
const settle = () => act(() => new Promise((resolve) => setTimeout(resolve, 0)));

describe("Greeting", () => {
  it("shows the push-to-talk shortcut before 2,000 dictated words", async () => {
    renderGreeting(1_999);
    expect(await screen.findByText(/to dictate anywhere/)).toBeInTheDocument();
    expect(screen.getByText("fn").tagName).toBe("KBD");
  });

  it("leaves the shortcut out from 2,000 words on", async () => {
    renderGreeting(2_000);
    expect(await screen.findByText("This week")).toBeInTheDocument();
    await settle();
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(/Dan/);
    expect(screen.queryByText(/to dictate anywhere/)).not.toBeInTheDocument();
  });
});
