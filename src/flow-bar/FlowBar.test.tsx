import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import {
  EVENTS,
  type Answer,
  type DictationState,
  type Notice,
  type TransformView,
} from "../lib/ipc";
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

    await setState({ phase: "recording", mode: "hold", command: false });
    await act(async () => emit(EVENTS.level, 0.8));
    expect(screen.getByRole("group", { name: "Recording" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Done" }));

    await setState({ phase: "recording", mode: "locked", command: false });
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

  it("marks a Command Mode session", async () => {
    backend();
    render(<FlowBar />);
    await screen.findByRole("button", { name: "Start dictation" });
    await setState({ phase: "recording", mode: "hold", command: true });
    expect(screen.getByRole("group", { name: "Command Mode" })).toHaveClass("pill-command");
  });

  it("shows a Command Mode answer until dismissed, with Copy", async () => {
    const calls = backend();
    render(<FlowBar />);
    await screen.findByRole("button", { name: "Start dictation" });
    const answer: Answer = { id: 1, question: "What's the capital of France?", text: "Paris." };
    await act(async () => emit(EVENTS.answer, answer));
    const panel = screen.getByRole("region", { name: "Command Mode answer" });
    expect(panel).toHaveTextContent("What's the capital of France?");
    expect(panel).toHaveTextContent("Paris.");
    fireEvent.click(screen.getByRole("button", { name: "Copy" }));
    await waitFor(() =>
      expect(calls).toContainEqual({
        cmd: "run_notice_action",
        args: { action: "copyLastTranscript" },
      }),
    );
    expect(screen.getByRole("button", { name: "Copied ✓" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Dismiss" }));
    expect(screen.queryByRole("region", { name: "Command Mode answer" })).toBeNull();
  });

  it("shows a transform's changes with Copy, Retry, Undo and Accept edits", async () => {
    const calls = backend();
    render(<FlowBar />);
    await screen.findByRole("button", { name: "Start dictation" });
    const view: TransformView = {
      id: 1,
      name: "Polish",
      original: "teh report is late",
      result: "The report is late.",
    };
    await act(async () => emit(EVENTS.changes, view));
    const card = screen.getByRole("region", { name: "Polish changes" });
    expect(card.querySelector("del")).toHaveTextContent("teh");
    expect(card.querySelector("ins")).toHaveTextContent("The");
    fireEvent.click(screen.getByRole("button", { name: "Copy" }));
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    fireEvent.click(screen.getByRole("button", { name: "Undo" }));
    await waitFor(() => {
      const actions = calls
        .filter((call) => call.cmd === "transform_action")
        .map((call) => (call.args as { action: string }).action);
      expect(actions).toEqual(["copy", "retry", "undo"]);
    });
    expect(screen.queryByRole("region", { name: "Polish changes" })).toBeNull();
    await act(async () => emit(EVENTS.changes, view));
    fireEvent.click(screen.getByRole("button", { name: "Accept edits" }));
    expect(screen.queryByRole("region", { name: "Polish changes" })).toBeNull();
  });
});
