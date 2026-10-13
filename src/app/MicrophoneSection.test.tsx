import { emit } from "@tauri-apps/api/event";
import { act, render, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { EVENTS } from "../lib/ipc";
import { MicrophoneSection } from "./MicrophoneSection";
import { MICROPHONES, mockBackend } from "./testBackend";

describe("the microphone meter", () => {
  it("stops while the window is closed to the tray and starts again when it opens", async () => {
    const calls: string[] = [];
    mockBackend({
      get_microphones: () => MICROPHONES,
      start_mic_preview: () => {
        calls.push("start");
        return null;
      },
      stop_mic_preview: () => {
        calls.push("stop");
        return null;
      },
    });
    render(<MicrophoneSection />);
    await waitFor(() => expect(calls).toEqual(["start"]));
    await act(() => emit(EVENTS.mainVisible, false));
    await waitFor(() => expect(calls).toEqual(["start", "stop"]));
    await act(() => emit(EVENTS.mainVisible, true));
    await waitFor(() => expect(calls).toEqual(["start", "stop", "start"]));
  });
});
