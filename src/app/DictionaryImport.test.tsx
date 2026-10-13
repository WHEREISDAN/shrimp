import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { DictionaryImportPreview } from "../lib/ipc";
import { DictionaryPage } from "./DictionaryPage";
import { chooseFile, entry, mockDictionary } from "./testBackend";

const WORDS = [entry({ id: 1, term: "Shrimp" }), entry({ id: 2, term: "Tauri" })];

const csvFile = (text: string, name = "words.csv") => new File([text], name, { type: "text/csv" });

describe("Dictionary CSV import", () => {
  const preview: DictionaryImportPreview = {
    added: [
      { term: "Whisper", misspelling: null },
      { term: "Tauri", misspelling: "tory" },
    ],
    duplicates: [{ term: "Shrimp", misspelling: null }],
    skipped: [{ line: 4, reason: "Too many columns" }],
  };

  it("previews the file, then imports it", async () => {
    const { argsOf } = mockDictionary(WORDS, {
      preview_dictionary_import: () => preview,
      import_dictionary: () => 2,
    });
    render(<DictionaryPage />);
    await screen.findByText("Tauri");
    expect(document.querySelector("input[type=file]")).toHaveAttribute("accept", ".csv,text/csv");

    chooseFile(csvFile("Whisper\ntory,Tauri\n"));

    const dialog = await screen.findByRole("dialog");
    expect(argsOf("preview_dictionary_import")).toEqual([{ csv: "Whisper\ntory,Tauri\n" }]);
    expect(within(dialog).getByText("Whisper")).toBeInTheDocument();
    expect(within(dialog).getByText("tory → Tauri")).toBeInTheDocument();
    expect(within(dialog).getByText("1 duplicate will be skipped")).toBeInTheDocument();
    expect(within(dialog).getByText("Line 4: Too many columns")).toBeInTheDocument();
    expect(
      within(dialog).getByText("One word per row, or misspelling,correction."),
    ).toBeInTheDocument();

    fireEvent.click(within(dialog).getByRole("button", { name: "Import 2 items" }));
    await waitFor(() =>
      expect(argsOf("import_dictionary")).toEqual([{ csv: "Whisper\ntory,Tauri\n" }]),
    );
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(argsOf("list_dictionary").length).toBeGreaterThan(1);
  });

  it("lists at most 50 new items", async () => {
    const added = Array.from({ length: 53 }, (_, index) => ({
      term: `term${index}`,
      misspelling: null,
    }));
    mockDictionary(WORDS, {
      preview_dictionary_import: () => ({ added, duplicates: [], skipped: [] }),
    });
    render(<DictionaryPage />);
    await screen.findByText("Tauri");
    chooseFile(csvFile("x"));

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getAllByRole("listitem")).toHaveLength(50);
    expect(within(dialog).getByText("and 3 more")).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Import 53 items" })).toBeInTheDocument();
  });

  it("says when everything already exists", async () => {
    mockDictionary(WORDS, {
      preview_dictionary_import: () => ({
        added: [],
        duplicates: [
          { term: "Shrimp", misspelling: null },
          { term: "Tauri", misspelling: null },
        ],
        skipped: [],
      }),
    });
    render(<DictionaryPage />);
    await screen.findByText("Tauri");
    chooseFile(csvFile("Shrimp\nTauri"));

    const dialog = await screen.findByRole("dialog");
    expect(
      within(dialog).getByText("All 2 items already exist in your dictionary"),
    ).toBeInTheDocument();
    fireEvent.click(within(dialog).getByRole("button", { name: "Done!" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("rejects the wrong file type and oversized files before reading them", async () => {
    const { argsOf } = mockDictionary(WORDS);
    render(<DictionaryPage />);
    await screen.findByText("Tauri");

    chooseFile(csvFile("a,b", "words.txt"));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Incorrect file type. Please upload a .csv file",
    );

    chooseFile(csvFile("x".repeat(3 * 1024 * 1024 + 1)));
    await waitFor(() =>
      expect(screen.getByRole("alert")).toHaveTextContent("File is too large (max 3MB)"),
    );
    expect(argsOf("preview_dictionary_import")).toEqual([]);
  });

  it("shows a backend refusal to preview", async () => {
    mockDictionary(WORDS, {
      preview_dictionary_import: () => {
        throw {
          code: "tooMany",
          message: "Cannot import more than 1,000 entries. Please try again.",
        };
      },
    });
    render(<DictionaryPage />);
    await screen.findByText("Tauri");
    chooseFile(csvFile("x"));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Cannot import more than 1,000 entries. Please try again.",
    );
  });

  it("shows a backend refusal to import inside the dialog", async () => {
    mockDictionary(WORDS, {
      preview_dictionary_import: () => preview,
      import_dictionary: () => {
        throw {
          code: "tooMany",
          message: "Cannot import more than 1,000 entries. Please try again.",
        };
      },
    });
    render(<DictionaryPage />);
    await screen.findByText("Tauri");
    chooseFile(csvFile("x"));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "Import 2 items" }));
    expect(await within(dialog).findByRole("alert")).toHaveTextContent(
      "Cannot import more than 1,000 entries. Please try again.",
    );
  });
});
