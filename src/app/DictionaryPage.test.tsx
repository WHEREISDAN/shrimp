import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { DictionaryEntry, Settings, SettingsPatch } from "../lib/ipc";
import { DictionaryPage } from "./DictionaryPage";
import { entry, mockDictionary, SETTINGS } from "./testBackend";

const WORDS: DictionaryEntry[] = [
  entry({ id: 1, term: "Shrimp", pinned: true, uses: 3 }),
  entry({ id: 2, term: "Tauri", uses: 1 }),
  entry({ id: 3, term: "Kubernetes", misspelling: "cooper netties", source: "imported" }),
];

describe("DictionaryPage", () => {
  it("lists entries with their markers and filters them by search", async () => {
    mockDictionary(WORDS);
    render(<DictionaryPage />);

    expect(await screen.findByText("Shrimp")).toBeInTheDocument();
    expect(screen.getByText("Starred")).toBeInTheDocument();
    expect(screen.getByText("Used 3 times")).toBeInTheDocument();
    expect(screen.getByText("Used 1 time")).toBeInTheDocument();
    expect(screen.getByText("cooper netties → Kubernetes")).toBeInTheDocument();
    expect(screen.getByText("Imported from file")).toBeInTheDocument();

    fireEvent.change(screen.getByRole("searchbox", { name: "Search dictionary" }), {
      target: { value: "COOPER" },
    });
    expect(screen.getByText("cooper netties → Kubernetes")).toBeInTheDocument();
    expect(screen.queryByText("Shrimp")).not.toBeInTheDocument();
  });

  it("shows the empty state", async () => {
    mockDictionary([]);
    render(<DictionaryPage />);
    expect(
      await screen.findByText(
        "No words yet. Add names, jargon and spellings Shrimp should get right.",
      ),
    ).toBeInTheDocument();
  });

  it("adds a word", async () => {
    const { argsOf } = mockDictionary(WORDS);
    render(<DictionaryPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Add word" }));

    const dialog = screen.getByRole("dialog", { name: "Add word" });
    expect(dialog).toHaveAttribute("aria-modal", "true");
    fireEvent.change(within(dialog).getByLabelText("Word"), { target: { value: "Whisper" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(argsOf("add_dictionary_entry")).toEqual([
        { input: { term: "Whisper", misspelling: null } },
      ]),
    );
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(await screen.findByText("Whisper")).toBeInTheDocument();
  });

  it("adds a correction", async () => {
    const { argsOf } = mockDictionary(WORDS);
    render(<DictionaryPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Add word" }));

    const dialog = screen.getByRole("dialog");
    expect(within(dialog).queryByLabelText("Misspelling")).not.toBeInTheDocument();
    fireEvent.change(within(dialog).getByLabelText("Word"), { target: { value: "Shrimp" } });
    fireEvent.click(within(dialog).getByRole("checkbox", { name: "Correct a misspelling" }));
    expect(
      within(dialog).getByText("What Shrimp hears, replaced by the word above."),
    ).toBeInTheDocument();
    fireEvent.change(within(dialog).getByLabelText("Misspelling"), {
      target: { value: "shrimpp" },
    });
    fireEvent.click(within(dialog).getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(argsOf("add_dictionary_entry")).toEqual([
        { input: { term: "Shrimp", misspelling: "shrimpp" } },
      ]),
    );
  });

  it("shows why the backend rejected a word and keeps the dialog open", async () => {
    mockDictionary(WORDS, {
      add_dictionary_entry: () => {
        throw { code: "duplicate", message: '"Shrimp" already exists in your personal dictionary' };
      },
    });
    render(<DictionaryPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Add word" }));
    const dialog = screen.getByRole("dialog");
    fireEvent.change(within(dialog).getByLabelText("Word"), { target: { value: "Shrimp" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Save" }));

    expect(await within(dialog).findByRole("alert")).toHaveTextContent(
      '"Shrimp" already exists in your personal dictionary',
    );
  });

  it("closes the dialog on Escape", async () => {
    mockDictionary(WORDS);
    render(<DictionaryPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Add word" }));
    fireEvent.keyDown(screen.getByLabelText("Word"), { key: "Escape" });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("edits an entry", async () => {
    const { argsOf } = mockDictionary(WORDS);
    render(<DictionaryPage />);
    await screen.findByText("Tauri");
    const row = screen.getByText("Tauri").closest("li");
    fireEvent.click(within(row as HTMLElement).getByRole("button", { name: "Edit" }));

    const dialog = screen.getByRole("dialog", { name: "Edit word" });
    expect(within(dialog).getByLabelText("Word")).toHaveValue("Tauri");
    fireEvent.change(within(dialog).getByLabelText("Word"), { target: { value: "Tauri 2" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(argsOf("update_dictionary_entry")).toEqual([
        { id: 2, input: { term: "Tauri 2", misspelling: null } },
      ]),
    );
  });

  it("stars and unstars an entry", async () => {
    const { argsOf } = mockDictionary(WORDS);
    render(<DictionaryPage />);
    await screen.findByText("Tauri");
    const starred = screen.getByRole("button", { name: "Unstar" });
    expect(starred).toHaveAttribute("aria-pressed", "true");
    fireEvent.click(starred);
    const [star] = screen.getAllByRole("button", { name: "Star" });
    fireEvent.click(star as HTMLElement);

    await waitFor(() =>
      expect(argsOf("set_dictionary_pinned")).toEqual([
        { id: 1, pinned: false },
        { id: 2, pinned: true },
      ]),
    );
  });

  it("sorts by the saved choice and saves a new one", async () => {
    let settings: Settings = { ...SETTINGS, dictionary: { autoLearn: true, sort: "alphabetical" } };
    const { argsOf } = mockDictionary(WORDS, {
      get_settings: () => settings,
      update_settings: (args) => {
        const patch = args.patch as SettingsPatch;
        settings = { ...settings, dictionary: { ...settings.dictionary, ...patch.dictionary } };
        return settings;
      },
    });
    render(<DictionaryPage />);
    const titles = () =>
      screen.getAllByRole("listitem").map((row) => row.querySelector(".list-title")?.textContent);
    const sortBy = await screen.findByRole("combobox", { name: "Sort by" });
    await waitFor(() => expect(sortBy).toHaveValue("alphabetical"));
    expect(titles()).toEqual(["cooper netties → Kubernetes", "Shrimp", "Tauri"]);
    expect(
      within(sortBy)
        .getAllByRole("option")
        .map((option) => option.textContent),
    ).toEqual(["Starred first", "Newest first", "Oldest first", "Alphabetical (A–Z)", "Most used"]);

    fireEvent.change(sortBy, { target: { value: "mostUsed" } });
    await waitFor(() =>
      expect(titles()).toEqual(["Shrimp", "Tauri", "cooper netties → Kubernetes"]),
    );
    expect(argsOf("update_settings")).toEqual([{ patch: { dictionary: { sort: "mostUsed" } } }]);
  });

  it("asks before deleting one entry", async () => {
    const { argsOf } = mockDictionary(WORDS);
    render(<DictionaryPage />);
    await screen.findByText("Tauri");
    const row = screen.getByText("Tauri").closest("li");
    fireEvent.click(within(row as HTMLElement).getByRole("button", { name: "Delete" }));

    const dialog = screen.getByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(argsOf("delete_dictionary_entries")).toEqual([]);

    fireEvent.click(within(row as HTMLElement).getByRole("button", { name: "Delete" }));
    fireEvent.click(screen.getByRole("button", { name: "Yes, delete it" }));
    await waitFor(() => expect(argsOf("delete_dictionary_entries")).toEqual([{ ids: [2] }]));
    await waitFor(() => expect(screen.queryByText("Tauri")).not.toBeInTheDocument());
  });

  it("deletes the selected entries with Cmd+Backspace after confirming", async () => {
    const { argsOf } = mockDictionary(WORDS);
    render(<DictionaryPage />);
    await screen.findByText("Tauri");

    fireEvent.keyDown(window, { key: "Backspace", metaKey: true });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("checkbox", { name: "Select Shrimp" }));
    fireEvent.click(screen.getByRole("checkbox", { name: "Select Tauri" }));
    expect(screen.getByText("2 selected")).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "Backspace", metaKey: true });
    const dialog = screen.getByRole("dialog", { name: "Delete 2 entries?" });
    fireEvent.click(within(dialog).getByRole("button", { name: "Delete" }));

    await waitFor(() => expect(argsOf("delete_dictionary_entries")).toEqual([{ ids: [1, 2] }]));
    await waitFor(() => expect(screen.queryByText("2 selected")).not.toBeInTheDocument());
  });

  it("leaves Cmd+Backspace to the search box while typing", async () => {
    mockDictionary(WORDS);
    render(<DictionaryPage />);
    await screen.findByText("Tauri");
    fireEvent.click(screen.getByRole("checkbox", { name: "Select Tauri" }));

    screen.getByRole("searchbox").focus();
    fireEvent.keyDown(window, { key: "Backspace", metaKey: true });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("deletes the selection from the Delete button", async () => {
    const { argsOf } = mockDictionary(WORDS);
    render(<DictionaryPage />);
    await screen.findByText("Tauri");
    fireEvent.click(screen.getByRole("checkbox", { name: "Select Kubernetes" }));
    const bar = screen.getByText("1 selected").parentElement as HTMLElement;
    fireEvent.click(within(bar).getByRole("button", { name: "Delete" }));
    fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(argsOf("delete_dictionary_entries")).toEqual([{ ids: [3] }]));
  });

  it("moves focus into the dialog, keeps it there and gives it back", async () => {
    mockDictionary(WORDS);
    render(<DictionaryPage />);
    const opener = await screen.findByRole("button", { name: "Add word" });
    expect(document.querySelector(".page")).not.toHaveAttribute("inert");
    opener.focus();
    fireEvent.click(opener);

    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByLabelText("Word")).toHaveFocus();
    expect(document.querySelector(".page")).toHaveAttribute("inert");
    expect(document.querySelector(".page")).toHaveAttribute("aria-hidden", "true");

    within(dialog).getByRole("button", { name: "Save" }).focus();
    fireEvent.keyDown(document.activeElement as Element, { key: "Tab" });
    expect(within(dialog).getByLabelText("Word")).toHaveFocus();
    fireEvent.keyDown(document.activeElement as Element, { key: "Tab", shiftKey: true });
    expect(within(dialog).getByRole("button", { name: "Save" })).toHaveFocus();

    fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(opener).toHaveFocus();
    expect(document.querySelector(".page")).not.toHaveAttribute("inert");
  });
});

describe("DictionaryPage auto-add", () => {
  it("shows learned words with a badge and turns auto-add off", async () => {
    const { argsOf } = mockDictionary([entry({ id: 1, term: "Nguyen", source: "learned" })], {
      get_settings: () => SETTINGS,
      update_settings: () => ({ ...SETTINGS, dictionary: { autoLearn: false } }),
    });
    render(<DictionaryPage />);
    expect(await screen.findByText("Learned")).toBeInTheDocument();
    fireEvent.click(await screen.findByRole("switch", { name: "Auto-add to dictionary" }));
    await waitFor(() =>
      expect(argsOf("update_settings")).toEqual([{ patch: { dictionary: { autoLearn: false } } }]),
    );
  });
});
