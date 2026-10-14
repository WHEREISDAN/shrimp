import { emit } from "@tauri-apps/api/event";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { EVENTS, type RecordingStatus, type ShortcutList } from "../lib/ipc";
import { ShortcutsDialog } from "./ShortcutsDialog";
import { mockBackend, SHORTCUTS } from "./testBackend";

function backend(list: ShortcutList = SHORTCUTS) {
  return mockBackend({
    get_shortcuts: () => list,
    record_shortcut: () => null,
    cancel_shortcut_recording: () => null,
    remove_shortcut: () => null,
    reset_shortcuts: () => null,
  });
}

const status = (value: RecordingStatus) => act(async () => emit(EVENTS.recording, value));

async function open() {
  let closed = 0;
  render(<ShortcutsDialog onClose={() => closed++} />);
  const dialog = await screen.findByRole("dialog", { name: "Shortcuts" });
  await within(dialog).findByText("Push to talk");
  return { dialog, closed: () => closed };
}

describe("ShortcutsDialog", () => {
  it("lists every action with its bindings and the empty ones", async () => {
    backend();
    const { dialog } = await open();
    expect(within(dialog).getByRole("button", { name: "Push to talk: fn. Change" })).toBeVisible();
    expect(within(dialog).getByRole("button", { name: "Click to add a shortcut" })).toBeVisible();
    expect(
      within(dialog).queryByRole("button", { name: "Remove shortcut fn from Push to talk" }),
      "required actions keep their last binding",
    ).toBeNull();
    expect(
      within(dialog).getByRole("button", { name: "Remove shortcut fn Space from Hands-free mode" }),
    ).toBeVisible();
    expect(
      within(dialog).queryByRole("button", { name: "Add another shortcut for Cancel dictation" }),
      "Cancel takes one",
    ).toBeNull();
  });

  it("hints at double-tapping push to talk on the Hands-free row, from the user's keys", async () => {
    let list = SHORTCUTS;
    mockBackend({ get_shortcuts: () => list });
    const { dialog } = await open();
    const handsFree = within(dialog).getByText("Hands-free mode").closest("li") as HTMLElement;
    expect(within(handsFree).getByText("Or double-tap fn")).toBeVisible();

    const rows = SHORTCUTS.rows.map((row) =>
      row.action === "pushToTalk"
        ? { ...row, shortcuts: ["fn", "Mouse 4"] }
        : row.action === "handsFree"
          ? { ...row, shortcuts: [] }
          : row,
    );
    list = { ...SHORTCUTS, rows };
    await act(async () => emit(EVENTS.settings, {}));
    expect(await within(dialog).findByText("Double-tap fn or Mouse 4")).toBeVisible();
    expect(within(dialog).getAllByText(/double-tap/i)).toHaveLength(1);
  });

  it("records a replacement through the hooks and says when it is saved", async () => {
    const { argsOf } = backend();
    const { dialog } = await open();
    fireEvent.click(within(dialog).getByRole("button", { name: "Push to talk: fn. Change" }));
    await waitFor(() =>
      expect(argsOf("record_shortcut")).toEqual([{ action: "pushToTalk", index: 0 }]),
    );
    expect(within(dialog).getByText("Press key or mouse button")).toBeVisible();
    await status({ state: "recording", action: "pushToTalk", preview: "⌃⌥" });
    expect(within(dialog).getByText("⌃⌥")).toBeVisible();
    await status({ state: "saved", action: "pushToTalk", shortcut: "⌃⌥", warning: null });
    expect(await within(dialog).findByText("Shortcut saved")).toBeVisible();
  });

  it("adds another binding and shows a conflict warning", async () => {
    const { argsOf } = backend();
    const { dialog } = await open();
    fireEvent.click(
      within(dialog).getByRole("button", { name: "Add another shortcut for Hands-free mode" }),
    );
    await waitFor(() =>
      expect(argsOf("record_shortcut")).toEqual([{ action: "handsFree", index: null }]),
    );
    const warning =
      "Warning: Your Hands-free mode shortcut is a part of a Push-to-talk shortcut. This may lead to conflicts.";
    await status({ state: "saved", action: "handsFree", shortcut: "⌃⌥", warning });
    expect(await within(dialog).findByText(warning)).toBeVisible();
  });

  it("shows why a shortcut was refused", async () => {
    backend();
    const { dialog } = await open();
    fireEvent.click(within(dialog).getByRole("button", { name: "Click to add a shortcut" }));
    await status({
      state: "failed",
      action: "pressEnter",
      message: "This shortcut is not allowed: ⌘V",
    });
    expect(await within(dialog).findByRole("alert")).toHaveTextContent(
      "This shortcut is not allowed: ⌘V",
    );
  });

  it("cancels with ✕, and keys never reach the dialog while recording", async () => {
    const { argsOf } = backend();
    const { dialog, closed } = await open();
    fireEvent.click(within(dialog).getByRole("button", { name: "Push to talk: fn. Change" }));
    fireEvent.keyDown(window, { key: "Escape" });
    expect(closed(), "Esc belongs to the recording").toBe(0);
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancel recording" }));
    await waitFor(() => expect(argsOf("cancel_shortcut_recording")).toHaveLength(1));
    expect(within(dialog).queryByText("Press key or mouse button")).toBeNull();
  });

  it("asks to finish recording before other changes", async () => {
    const { argsOf } = backend();
    const { dialog } = await open();
    fireEvent.click(within(dialog).getByRole("button", { name: "Push to talk: fn. Change" }));
    fireEvent.click(within(dialog).getByRole("button", { name: "Reset to default" }));
    expect(await within(dialog).findByRole("alert")).toHaveTextContent(
      "Finish or cancel the recording first.",
    );
    expect(argsOf("reset_shortcuts")).toEqual([]);
  });

  it("removes a binding with its trash icon", async () => {
    const { argsOf } = backend();
    const { dialog } = await open();
    fireEvent.click(
      within(dialog).getByRole("button", { name: "Remove shortcut fn Space from Hands-free mode" }),
    );
    await waitFor(() =>
      expect(argsOf("remove_shortcut")).toEqual([{ action: "handsFree", index: 0 }]),
    );
  });

  it("resets to the defaults after a confirmation, and Done closes", async () => {
    const { argsOf } = backend();
    const { dialog, closed } = await open();
    fireEvent.click(within(dialog).getByRole("button", { name: "Reset to default" }));
    expect(within(dialog).getByText(/This can't be undone/)).toBeVisible();
    fireEvent.click(within(dialog).getByRole("button", { name: "Reset" }));
    await waitFor(() => expect(argsOf("reset_shortcuts")).toHaveLength(1));
    fireEvent.click(within(dialog).getByRole("button", { name: "Done" }));
    expect(closed()).toBe(1);
  });

  it("explains Secure Keyboard Entry", async () => {
    backend({ ...SHORTCUTS, secureInput: "1Password" });
    const { dialog } = await open();
    expect(
      within(dialog).getByText(/Secure Keyboard Entry is on \(macOS names 1Password\)/),
    ).toBeVisible();
  });
});
