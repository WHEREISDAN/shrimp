import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { Snippet } from "../lib/ipc";
import { chooseFile, mockBackend, snippet, type Args } from "./testBackend";
import { SnippetsPage } from "./SnippetsPage";

const SNIPPETS: Snippet[] = [
  snippet({
    id: 1,
    trigger: "my address",
    expansion: "**Alex** lives at [home](https://example.com) with \\*stars\\*",
  }),
  snippet({ id: 2, trigger: "sign off", expansion: "Thanks,\nAlex" }),
];

function backend(extra: Record<string, (args: Args) => unknown> = {}) {
  let snippets = SNIPPETS;
  return mockBackend({
    list_snippets: () => snippets,
    add_snippet: (args) => {
      const input = args.input as { trigger: string; expansion: string };
      const saved = snippet({ id: 9, ...input });
      snippets = [...snippets, saved];
      return { snippet: saved, warning: null };
    },
    update_snippet: () => ({ snippet: SNIPPETS[0], warning: null }),
    delete_snippets: (args) => {
      const ids = args.ids as number[];
      snippets = snippets.filter((candidate) => !ids.includes(candidate.id));
      return ids.length;
    },
    ...extra,
  });
}

const jsonFile = (text: string, name = "snippets.json") =>
  new File([text], name, { type: "application/json" });

async function openEditor() {
  fireEvent.click(await screen.findByRole("button", { name: "Add snippet" }));
  const dialog = screen.getByRole("dialog", { name: "Add snippet" });
  const expansion = within(dialog).getByLabelText("Expansion") as HTMLTextAreaElement;
  return { dialog, trigger: within(dialog).getByLabelText("Snippet"), expansion };
}

describe("SnippetsPage", () => {
  it("lists triggers with a one-line plain preview", async () => {
    backend();
    render(<SnippetsPage />);
    expect(await screen.findByText("my address")).toBeInTheDocument();
    expect(screen.getByText("Alex lives at home with *stars*")).toBeInTheDocument();
    expect(screen.getByText("Thanks, Alex")).toBeInTheDocument();
  });

  it("shows the empty state", async () => {
    mockBackend({ list_snippets: () => [] });
    render(<SnippetsPage />);
    expect(
      await screen.findByText("No snippets yet. Say a short trigger, get your saved text."),
    ).toBeInTheDocument();
  });

  it("saves with Cmd+Enter and shows the similar-trigger warning", async () => {
    const { argsOf } = backend({
      add_snippet: (args) => ({
        snippet: snippet({ id: 9, ...(args.input as object) }),
        warning: "Similar to your snippet “my address”",
      }),
    });
    render(<SnippetsPage />);
    const { trigger, expansion } = await openEditor();
    fireEvent.change(trigger, { target: { value: "my adress" } });
    fireEvent.change(expansion, { target: { value: "12 Main St" } });
    expect(screen.getByText("10/4000")).toBeInTheDocument();
    fireEvent.keyDown(expansion, { key: "Enter", metaKey: true });

    await waitFor(() =>
      expect(argsOf("add_snippet")).toEqual([
        { input: { trigger: "my adress", expansion: "12 Main St" } },
      ]),
    );
    expect(await screen.findByRole("status")).toHaveTextContent(
      "Similar to your snippet “my address”",
    );
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("hides the warning after ten seconds", async () => {
    const timers = vi.spyOn(globalThis, "setTimeout");
    backend({
      add_snippet: () => ({ snippet: SNIPPETS[0], warning: "Similar to “my address”" }),
    });
    render(<SnippetsPage />);
    const { trigger, expansion } = await openEditor();
    fireEvent.change(trigger, { target: { value: "x" } });
    fireEvent.change(expansion, { target: { value: "y" } });
    fireEvent.keyDown(expansion, { key: "Enter", metaKey: true });
    expect(await screen.findByRole("status")).toBeInTheDocument();

    const expiry = () => timers.mock.calls.find(([, delay]) => delay === 10_000)?.[0];
    await waitFor(() => expect(expiry()).toBeTypeOf("function"));
    act(() => (expiry() as () => void)());
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("also saves with Cmd+Enter from the snippet field", async () => {
    const { argsOf } = backend();
    render(<SnippetsPage />);
    const { trigger, expansion } = await openEditor();
    fireEvent.change(trigger, { target: { value: "hi" } });
    fireEvent.change(expansion, { target: { value: "hello" } });
    fireEvent.keyDown(trigger, { key: "Enter", metaKey: true });
    await waitFor(() => expect(argsOf("add_snippet")).toHaveLength(1));
  });

  it("shows backend validation inside the dialog", async () => {
    backend({
      add_snippet: () => {
        throw { code: "invalid", message: "Please enter a snippet and its expansion" };
      },
    });
    render(<SnippetsPage />);
    const { dialog } = await openEditor();
    fireEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    expect(await within(dialog).findByRole("alert")).toHaveTextContent(
      "Please enter a snippet and its expansion",
    );
  });

  it("says when formatting would make the expansion too long", async () => {
    const { argsOf } = backend();
    render(<SnippetsPage />);
    const { dialog, expansion } = await openEditor();
    fireEvent.change(expansion, { target: { value: "x".repeat(3999) } });
    expansion.setSelectionRange(0, 3999);
    fireEvent.keyDown(expansion, { key: "b", metaKey: true });
    expect(expansion).toHaveValue("x".repeat(3999));
    expect(within(dialog).getByRole("alert")).toHaveTextContent(
      "This expansion is too long. Please shorten it and try again.",
    );
    fireEvent.change(expansion, { target: { value: "short" } });
    expect(within(dialog).queryByRole("alert")).not.toBeInTheDocument();
    expect(argsOf("add_snippet")).toEqual([]);
  });

  it("turns the selection into a link with Cmd+K and selects the address", async () => {
    backend();
    render(<SnippetsPage />);
    const { expansion } = await openEditor();

    fireEvent.change(expansion, { target: { value: "see docs" } });
    expansion.setSelectionRange(4, 8);
    fireEvent.keyDown(expansion, { key: "k", metaKey: true });
    expect(expansion).toHaveValue("see [docs](https://)");
    expect(expansion.value.slice(expansion.selectionStart, expansion.selectionEnd)).toBe(
      "https://",
    );

    fireEvent.change(expansion, { target: { value: "" } });
    fireEvent.keyDown(expansion, { key: "k", metaKey: true });
    expect(expansion).toHaveValue("[link text](https://)");
  });

  it("wraps the selection in bold and italic marks", async () => {
    backend();
    render(<SnippetsPage />);
    const { dialog, expansion } = await openEditor();

    fireEvent.change(expansion, { target: { value: "say hello" } });
    expansion.setSelectionRange(4, 9);
    fireEvent.keyDown(expansion, { key: "b", metaKey: true });
    expect(expansion).toHaveValue("say **hello**");

    fireEvent.change(expansion, { target: { value: "" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Italic" }));
    expect(expansion).toHaveValue("**");
    expect(expansion.selectionStart).toBe(1);
    fireEvent.change(expansion, { target: { value: "x" } });
    expansion.setSelectionRange(0, 1);
    fireEvent.keyDown(expansion, { key: "i", metaKey: true });
    expect(expansion).toHaveValue("*x*");
  });

  it("edits a snippet and reports that nothing changed", async () => {
    const { argsOf } = backend({
      update_snippet: () => {
        throw { code: "unchanged", message: "No changes made" };
      },
    });
    render(<SnippetsPage />);
    const row = (await screen.findByText("sign off")).closest("li") as HTMLElement;
    fireEvent.click(within(row).getByRole("button", { name: "Edit" }));

    const dialog = screen.getByRole("dialog", { name: "Edit snippet" });
    expect(within(dialog).getByLabelText("Snippet")).toHaveValue("sign off");
    fireEvent.click(within(dialog).getByRole("button", { name: "Save changes" }));

    expect(await within(dialog).findByRole("alert")).toHaveTextContent("No changes made");
    expect(argsOf("update_snippet")).toEqual([
      { id: 2, input: { trigger: "sign off", expansion: "Thanks,\nAlex" } },
    ]);
  });

  it("asks before deleting", async () => {
    const { argsOf } = backend();
    render(<SnippetsPage />);
    const row = (await screen.findByText("sign off")).closest("li") as HTMLElement;
    fireEvent.click(within(row).getByRole("button", { name: "Delete" }));
    fireEvent.click(screen.getByRole("button", { name: "Yes, delete it" }));
    await waitFor(() => expect(argsOf("delete_snippets")).toEqual([{ ids: [2] }]));
    await waitFor(() => expect(screen.queryByText("sign off")).not.toBeInTheDocument());
  });
});

describe("SnippetsPage JSON import", () => {
  const preview = {
    added: [{ trigger: "new one", expansion: "a" }],
    duplicates: [{ trigger: "sign off", expansion: "b" }],
    skipped: [{ line: 0, reason: "Missing text" }],
  };

  it("previews and imports", async () => {
    const { argsOf } = backend({
      preview_snippet_import: () => preview,
      import_snippets: () => 1,
    });
    render(<SnippetsPage />);
    await screen.findByText("sign off");
    expect(document.querySelector("input[type=file]")).toHaveAttribute(
      "accept",
      ".json,application/json",
    );

    chooseFile(jsonFile("[]"));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("new one")).toBeInTheDocument();
    expect(within(dialog).getByText("1 duplicate will be skipped")).toBeInTheDocument();
    expect(
      within(dialog).getByText('A JSON array of {"name", "text"} objects.'),
    ).toBeInTheDocument();
    fireEvent.click(within(dialog).getByRole("button", { name: "Import 1 snippet" }));

    await waitFor(() => expect(argsOf("import_snippets")).toEqual([{ json: "[]" }]));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("says when every snippet already exists", async () => {
    backend({
      preview_snippet_import: () => ({ added: [], duplicates: preview.duplicates, skipped: [] }),
    });
    render(<SnippetsPage />);
    await screen.findByText("sign off");
    chooseFile(jsonFile("[]"));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("This snippet already exists")).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Done!" })).toBeInTheDocument();
  });

  it("rejects the wrong file type and oversized files", async () => {
    backend();
    render(<SnippetsPage />);
    await screen.findByText("sign off");

    chooseFile(jsonFile("[]", "snippets.csv"));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Incorrect file type. Please upload a .json file",
    );
    chooseFile(jsonFile("x".repeat(3 * 1024 * 1024 + 1)));
    await waitFor(() =>
      expect(screen.getByRole("alert")).toHaveTextContent("File is too large (max 3MB)"),
    );
  });
});
