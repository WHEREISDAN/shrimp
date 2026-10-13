import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import {
  EVENTS,
  type Answer,
  type DictationState,
  type Languages,
  type Notice,
  type Settings,
  type TransformView,
} from "../lib/ipc";
import { MICROPHONES, SETTINGS } from "../app/testBackend";
import { FlowBar } from "./FlowBar";

const LANGUAGES: Languages = {
  options: [
    { id: "en-GB", name: "English (UK)", group: "en" },
    { id: "fr", name: "French", group: null },
  ],
  chosen: { autoDetect: false, selected: ["en-GB", "fr"], active: null },
};

function backend(languages: Languages = LANGUAGES, flowBar: Partial<Settings["flowBar"]> = {}) {
  const calls: { cmd: string; args: unknown }[] = [];
  mockIPC(
    (cmd, args) => {
      calls.push({ cmd, args });
      if (cmd === "get_dictation_state") {
        return { phase: "idle" };
      }
      if (cmd === "get_languages") {
        return languages;
      }
      if (cmd === "get_settings") {
        return { ...SETTINGS, flowBar: { ...SETTINGS.flowBar, ...flowBar } };
      }
      if (cmd === "get_microphones") {
        return MICROPHONES;
      }
      if (cmd === "set_active_language") {
        return { ...languages.chosen, active: (args as { id: string | null }).id };
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
      actions: ["copyLastTranscript"],
      category: "status",
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
    // The button confirms with a ✓, then the notice goes (NOT-02).
    expect(screen.getByRole("button", { name: "Done" })).toHaveTextContent("✓");
    await waitFor(() => expect(screen.queryByText(/No text field/)).not.toBeInTheDocument());
  });

  it("queues notices, and an error interrupts", async () => {
    backend();
    render(<FlowBar />);
    await screen.findByRole("button", { name: "Start dictation" });
    const notice = (id: number, level: Notice["level"], message: string): Notice => ({
      id,
      level,
      message,
      actions: [],
      category: "status",
    });
    await act(async () => emit(EVENTS.notice, notice(1, "info", "First")));
    await act(async () => emit(EVENTS.notice, notice(2, "info", "Second")));
    expect(screen.getByText("First")).toBeInTheDocument();
    expect(screen.queryByText("Second")).not.toBeInTheDocument();

    await act(async () => emit(EVENTS.notice, notice(3, "error", "Broken")));
    expect(screen.getByRole("alert")).toHaveTextContent("Broken");
    expect(screen.queryByText("First")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Dismiss notification" }));
    expect(screen.getByText("First")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Dismiss notification" }));
    expect(screen.getByText("Second")).toBeInTheDocument();
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

  it("offers one-click language switching on hover when there are several languages", async () => {
    const calls = backend();
    render(<FlowBar />);
    await screen.findByRole("button", { name: "Start dictation" });
    expect(screen.queryByRole("group", { name: "Dictation language" })).toBeNull();
    await act(async () => emit(EVENTS.flowBarHover, true));
    const french = await screen.findByRole("button", { name: "French" });
    expect(screen.getByRole("button", { name: "Auto" })).toHaveAttribute("aria-pressed", "true");
    fireEvent.click(french);
    await waitFor(() =>
      expect(calls).toContainEqual({ cmd: "set_active_language", args: { id: "fr" } }),
    );
    await waitFor(() => expect(french).toHaveAttribute("aria-pressed", "true"));
    await act(async () => emit(EVENTS.flowBarHover, false));
    expect(screen.queryByRole("group", { name: "Dictation language" })).toBeNull();
  });

  it("shows no language switch with one language", async () => {
    backend({ ...LANGUAGES, chosen: { autoDetect: false, selected: ["fr"], active: null } });
    const { container } = render(<FlowBar />);
    await screen.findByRole("button", { name: "Start dictation" });
    fireEvent.mouseEnter(container.querySelector(".flow-bar") as Element);
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(screen.queryByRole("group", { name: "Dictation language" })).toBeNull();
  });

  it("drops a notice its cause no longer applies to", async () => {
    backend();
    render(<FlowBar />);
    await screen.findByRole("button", { name: "Start dictation" });
    const notice: Notice = {
      id: 9,
      level: "warning",
      message: "No audio",
      actions: [],
      category: "status",
    };
    await act(async () => emit(EVENTS.notice, notice));
    expect(screen.getByText("No audio")).toBeInTheDocument();
    await act(async () => emit(EVENTS.dismiss, 9));
    expect(screen.queryByText("No audio")).toBeNull();
  });

  it("hides the resting bar when it shouldn't show at all times, but not while dictating", async () => {
    backend(LANGUAGES, { alwaysShow: false });
    render(<FlowBar />);
    await waitFor(() =>
      expect(screen.queryByRole("button", { name: "Start dictation" })).toBeNull(),
    );
    await setState({ phase: "recording", mode: "hold", command: false });
    expect(screen.getByRole("group", { name: "Recording" })).toBeInTheDocument();
  });

  it("stays hidden for the hour after Hide for 1 hour", async () => {
    backend(LANGUAGES, { hiddenUntil: Math.floor(Date.now() / 1000) + 3600 });
    render(<FlowBar />);
    await waitFor(() =>
      expect(screen.queryByRole("button", { name: "Start dictation" })).toBeNull(),
    );
  });

  it("opens its menu on right-click", async () => {
    const calls = backend();
    const { container } = render(<FlowBar />);
    await screen.findByRole("button", { name: "Start dictation" });
    fireEvent.contextMenu(container.querySelector(".flow-bar") as Element);
    const menu = screen.getByRole("menu", { name: "Flow Bar menu" });
    for (const item of [
      "Paste last transcript",
      "Transcript history",
      "Settings",
      "Hide for 1 hour",
    ]) {
      expect(screen.getByRole("menuitem", { name: item })).toBeInTheDocument();
    }
    fireEvent.click(screen.getByRole("menuitem", { name: "Microphone ›" }));
    fireEvent.click(await screen.findByRole("menuitemradio", { name: "Yeti" }));
    await waitFor(() =>
      expect(calls).toContainEqual({ cmd: "select_microphone", args: { id: "usb" } }),
    );
    expect(menu).not.toBeInTheDocument();
    fireEvent.contextMenu(container.querySelector(".flow-bar") as Element);
    fireEvent.click(screen.getByRole("menuitem", { name: "Hide for 1 hour" }));
    await waitFor(() =>
      expect(calls).toContainEqual({ cmd: "flow_bar_action", args: { action: "hide" } }),
    );
  });

  it("drags the resting bar instead of starting a dictation", async () => {
    const calls = backend();
    render(<FlowBar />);
    const bar = await screen.findByRole("button", { name: "Start dictation" });
    bar.setPointerCapture = () => undefined;
    fireEvent.pointerDown(bar, { button: 0, pointerId: 1, screenX: 100, screenY: 100 });
    fireEvent.pointerMove(bar, { pointerId: 1, screenX: 160, screenY: 40 });
    fireEvent.pointerUp(bar, { pointerId: 1, screenX: 160, screenY: 40 });
    fireEvent.click(bar);
    await waitFor(() =>
      expect(calls).toContainEqual({ cmd: "flow_bar_drag", args: { ended: true } }),
    );
    expect(calls.some((call) => call.cmd === "flow_bar_control")).toBe(false);
  });
});
