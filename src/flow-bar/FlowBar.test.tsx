import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { EVENTS, type DictationState, type Notice } from "../lib/ipc";
import { FlowBar } from "./FlowBar";

function backend() {
  const calls: { cmd: string; args: unknown }[] = [];
  mockIPC(
    (cmd, args) => {
      calls.push({ cmd, args });
      if (cmd === "get_dictation_state") {
        return { phase: "idle" };
      }
      return null;
    },
    { shouldMockEvents: true },
  );
  return calls;
}

async function setState(state: DictationState) {
  await act(async () => emit(EVENTS.state, state));
}

describe("FlowBar", () => {
  it("starts hands-free dictation when the resting bar is clicked", async () => {
    const calls = backend();
    render(<FlowBar />);
    fireEvent.click(await screen.findByRole("button", { name: "Start dictation" }));
    await waitFor(() =>
      expect(calls).toContainEqual({ cmd: "flow_bar_control", args: { control: "start" } }),
    );
  });

  it("shows the waveform and controls while recording", async () => {
    const calls = backend();
    render(<FlowBar />);
    await screen.findByRole("button", { name: "Start dictation" });

    await setState({ phase: "recording", mode: "hold" });
    await act(async () => emit(EVENTS.level, 0.8));
    expect(screen.getByRole("group", { name: "Recording" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Done" }));

    await setState({ phase: "recording", mode: "locked" });
    fireEvent.click(screen.getByRole("button", { name: "Stop" }));
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    await waitFor(() => {
      const controls = calls
        .filter((call) => call.cmd === "flow_bar_control")
        .map((call) => (call.args as { control: string }).control);
      expect(controls).toEqual(["confirm", "stop", "cancel"]);
    });
    const bars = document.querySelectorAll<HTMLElement>(".bar");
    expect(bars[bars.length - 1]?.style.transform).toBe("scaleY(0.8)");
  });

  it("shows processing", async () => {
    backend();
    render(<FlowBar />);
    await screen.findByRole("button", { name: "Start dictation" });
    await setState({ phase: "processing" });
    expect(screen.getByRole("status", { name: "Transcribing" })).toBeInTheDocument();
  });

  it("shows notices with their action and reports readiness", async () => {
    const calls = backend();
    render(<FlowBar />);
    await waitFor(() => expect(calls.map((call) => call.cmd)).toContain("flow_bar_ready"));

    const notice: Notice = {
      id: 7,
      level: "warning",
      message: "No text field is focused. Your transcript is on the clipboard.",
      action: "copyLastTranscript",
    };
    await act(async () => emit(EVENTS.notice, notice));
    expect(screen.getByRole("status")).toHaveTextContent("No text field is focused.");

    fireEvent.click(screen.getByRole("button", { name: "Copy" }));
    await waitFor(() =>
      expect(calls).toContainEqual({
        cmd: "run_notice_action",
        args: { action: "copyLastTranscript" },
      }),
    );
    expect(screen.queryByText(/No text field/)).not.toBeInTheDocument();
  });
});
