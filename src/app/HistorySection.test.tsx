import { emit } from "@tauri-apps/api/event";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { EVENTS, type HistoryItem, type HistoryList } from "../lib/ipc";
import { HistorySection } from "./HistorySection";
import { HistoryStorageSetting } from "./HistoryStorageSetting";
import { mockBackend } from "./testBackend";

const now = new Date();
/** The first seconds of today, so the row is always "Today", minus `id` ms. */
const TODAY = new Date(now.getFullYear(), now.getMonth(), now.getDate(), 0, 0, 5).getTime();
const YESTERDAY_NOON = TODAY - 12 * 3_600_000;

const item = (overrides: Partial<HistoryItem> & { id: number }): HistoryItem => ({
  createdAt: TODAY - overrides.id,
  status: "inserted",
  appName: "TextEdit",
  rawText: "um ship it",
  finalText: "Ship it.",
  formatted: true,
  showRaw: false,
  language: "en",
  audioMs: 3000,
  wordCount: 2,
  error: null,
  hasAudio: true,
  retry: null,
  ...overrides,
});

function backend(list: HistoryList) {
  let current = list;
  return mockBackend({
    list_history: (args) => {
      const query = String(args.query).toLowerCase();
      return {
        ...current,
        entries: current.entries.filter((entry) => entry.finalText.toLowerCase().includes(query)),
      };
    },
    copy_history: () => null,
    set_history_raw: (args) => {
      current = {
        ...current,
        entries: current.entries.map((entry) =>
          entry.id === args.id ? { ...entry, showRaw: Boolean(args.raw) } : entry,
        ),
      };
      return current.entries.find((entry) => entry.id === args.id);
    },
    retry_history: () => null,
    delete_history: (args) => {
      current = { ...current, entries: current.entries.filter((entry) => entry.id !== args.id) };
      return null;
    },
    extract_history_audio: () => "/Users/me/Downloads/Shrimp 2026-10-13 09.00.00.wav",
    set_history_storage: () => null,
  });
}

const rows = () => document.querySelectorAll<HTMLElement>("[data-history-id]");

describe("HistorySection", () => {
  it("groups dictations by day with their app and language, and copies on click", async () => {
    const { argsOf } = backend({
      entries: [
        item({ id: 1 }),
        item({
          id: 30,
          createdAt: YESTERDAY_NOON,
          finalText: "Older one.",
          formatted: false,
          rawText: "older one",
        }),
      ],
      disabled: false,
    });
    render(<HistorySection />);
    expect(await screen.findByRole("heading", { name: "Yesterday" })).toBeInTheDocument();
    const today = screen.getByRole("list", { name: "Today" });
    expect(within(today).getByText("Ship it.")).toBeInTheDocument();
    expect(within(today).getByText("English")).toBeInTheDocument();
    expect(screen.getByText("Unformatted")).toBeInTheDocument();

    fireEvent.click(screen.getByText("Ship it."));
    await waitFor(() => expect(argsOf("copy_history")).toEqual([{ id: 1 }]));
    expect(await screen.findByText("Copied to the clipboard")).toBeInTheDocument();
  });

  it("moves between rows with j/k and the arrows, and Enter copies", async () => {
    const { argsOf } = backend({
      entries: [item({ id: 1 }), item({ id: 2, finalText: "Second." })],
      disabled: false,
    });
    render(<HistorySection />);
    await screen.findByText("Second.");
    const [first, second] = rows();
    first?.focus();
    fireEvent.keyDown(first!, { key: "j" });
    expect(second).toHaveFocus();
    expect(second).toHaveAttribute("tabindex", "0");
    expect(first).toHaveAttribute("tabindex", "-1");
    fireEvent.keyDown(second!, { key: "ArrowUp" });
    expect(first).toHaveFocus();
    fireEvent.click(first!);
    await waitFor(() => expect(argsOf("copy_history")).toEqual([{ id: 1 }]));
  });

  it("focuses search with Cmd+F or / and filters", async () => {
    backend({ entries: [item({ id: 1 }), item({ id: 2, finalText: "Coffee." })], disabled: false });
    render(<HistorySection />);
    await screen.findByText("Coffee.");
    fireEvent.keyDown(document.body, { key: "f", metaKey: true });
    const search = screen.getByRole("searchbox", { name: "Search history" });
    expect(search).toHaveFocus();
    search.blur();
    fireEvent.keyDown(document.body, { key: "/" });
    expect(search).toHaveFocus();
    fireEvent.change(search, { target: { value: "coffee" } });
    await waitFor(() => expect(rows()).toHaveLength(1));
  });

  it("offers Undo AI edit, then Redo AI edit", async () => {
    const { argsOf } = backend({ entries: [item({ id: 1 })], disabled: false });
    render(<HistorySection />);
    await screen.findByText("Ship it.");
    fireEvent.click(screen.getByRole("button", { name: "More options" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "Undo AI edit" }));
    await waitFor(() => expect(argsOf("set_history_raw")).toEqual([{ id: 1, raw: true }]));
    await act(() => emit(EVENTS.history));
    expect(await screen.findByText("um ship it")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "More options" }));
    expect(screen.getByRole("menuitem", { name: "Redo AI edit" })).toBeInTheDocument();
  });

  it("retries a failed dictation and recovers an interrupted one", async () => {
    const { argsOf } = backend({
      entries: [
        item({ id: 1, status: "failed", error: "Groq rate limit reached.", retry: "retry" }),
        item({ id: 2, status: "interrupted", retry: "recover", finalText: "" }),
      ],
      disabled: false,
    });
    render(<HistorySection />);
    expect(await screen.findByText("Groq rate limit reached.")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    fireEvent.click(screen.getByRole("button", { name: "Recover" }));
    await waitFor(() => expect(argsOf("retry_history")).toEqual([{ id: 1 }, { id: 2 }]));
  });

  it("deletes a transcript after confirming", async () => {
    const { argsOf } = backend({ entries: [item({ id: 1 })], disabled: false });
    render(<HistorySection />);
    await screen.findByText("Ship it.");
    fireEvent.click(screen.getByRole("button", { name: "More options" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "Delete transcript" }));
    const dialog = screen.getByRole("dialog", {
      name: "Are you sure you want to delete this transcript?",
    });
    fireEvent.click(within(dialog).getByRole("button", { name: "Yes, delete it" }));
    await waitFor(() => expect(argsOf("delete_history")).toEqual([{ id: 1 }]));
    expect(await screen.findByText("Transcript deleted")).toBeInTheDocument();
  });

  it("extracts the audio to Downloads", async () => {
    backend({ entries: [item({ id: 1 })], disabled: false });
    render(<HistorySection />);
    await screen.findByText("Ship it.");
    fireEvent.click(screen.getByRole("button", { name: "More options" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "Extract audio" }));
    expect(await screen.findByText(/Saved to .*Downloads/)).toBeInTheDocument();
  });

  it("says when history is disabled or empty", async () => {
    backend({ entries: [], disabled: true });
    const { unmount } = render(<HistorySection />);
    expect(await screen.findByText("History disabled")).toBeInTheDocument();
    unmount();
    backend({ entries: [], disabled: false });
    render(<HistorySection />);
    expect(await screen.findByText("No dictations yet")).toBeInTheDocument();
  });
});

describe("HistoryStorageSetting", () => {
  it("needs “delete all” typed before it stops storing", async () => {
    const { argsOf } = backend({ entries: [], disabled: false });
    render(<HistoryStorageSetting value="local" />);
    fireEvent.click(screen.getByRole("radio", { name: "Never store data locally" }));
    const dialog = screen.getByRole("dialog", { name: "Delete all history?" });
    const confirm = within(dialog).getByRole("button", { name: "Delete all data" });
    expect(confirm).toBeDisabled();
    fireEvent.change(within(dialog).getByLabelText("Confirmation"), {
      target: { value: "delete all" },
    });
    fireEvent.click(confirm);
    await waitFor(() =>
      expect(argsOf("set_history_storage")).toEqual([
        { storage: "never", confirmation: "delete all" },
      ]),
    );
  });

  it("asks before turning on auto-delete", async () => {
    const { argsOf } = backend({ entries: [], disabled: false });
    render(<HistoryStorageSetting value="local" />);
    fireEvent.click(screen.getByRole("radio", { name: "Auto-delete local data every 24hrs" }));
    fireEvent.click(screen.getByRole("button", { name: "Enable auto-delete" }));
    await waitFor(() =>
      expect(argsOf("set_history_storage")).toEqual([
        { storage: "autoDelete", confirmation: null },
      ]),
    );
  });
});
