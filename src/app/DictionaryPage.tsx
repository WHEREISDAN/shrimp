import { useCallback, useEffect, useMemo, useState } from "react";
import {
  deleteDictionaryEntries,
  errorMessage,
  importDictionary,
  listDictionary,
  previewDictionaryImport,
  setDictionaryPinned,
  type DictionaryEntry,
} from "../lib/ipc";
import { ConfirmDialog } from "./ConfirmDialog";
import { DictionaryDialog } from "./DictionaryDialog";
import { DictionaryRow } from "./DictionaryRow";
import { ImportDialog, ImportFileInput } from "./ImportDialog";
import { Page } from "./Page";
import { useFileImport } from "./useFileImport";

type Dialog =
  | { kind: "edit"; entry: DictionaryEntry | null }
  | { kind: "delete"; ids: number[]; bulk: boolean };

const NON_TEXT_INPUTS = new Set(["checkbox", "radio", "button", "submit", "reset", "file"]);

/** Whether typing would land in the focused element, so Cmd+Backspace should edit text instead. */
function isTextEntry(element: Element | null): boolean {
  if (element instanceof HTMLInputElement) {
    return !NON_TEXT_INPUTS.has(element.type);
  }
  return (
    element instanceof HTMLTextAreaElement ||
    element instanceof HTMLSelectElement ||
    element?.closest("[contenteditable]:not([contenteditable='false'])") != null
  );
}

const matches = (entry: DictionaryEntry, query: string) =>
  entry.term.toLowerCase().includes(query) ||
  (entry.misspelling?.toLowerCase().includes(query) ?? false);

export function DictionaryPage() {
  const [entries, setEntries] = useState<DictionaryEntry[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<ReadonlySet<number>>(new Set());
  const [dialog, setDialog] = useState<Dialog | null>(null);

  const reload = useCallback(
    () =>
      listDictionary()
        .then(setEntries)
        .catch((err: unknown) => setError(errorMessage(err))),
    [],
  );
  useEffect(() => {
    void reload();
  }, [reload]);

  const importer = useFileImport({
    extension: ".csv",
    preview: async (csv) => {
      const { added, duplicates, skipped } = await previewDictionaryImport(csv);
      return {
        added: added.map((item) =>
          item.misspelling === null ? item.term : `${item.misspelling} → ${item.term}`,
        ),
        duplicates: duplicates.length,
        skipped,
      };
    },
    commit: importDictionary,
    onImported: () => void reload(),
  });

  const needle = query.trim().toLowerCase();
  const visible = useMemo(
    () => (entries ?? []).filter((entry) => matches(entry, needle)),
    [entries, needle],
  );
  const selectedIds = useMemo(
    () => visible.filter((entry) => selected.has(entry.id)).map((entry) => entry.id),
    [visible, selected],
  );
  const modalOpen = dialog !== null || importer.session !== null;

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!event.metaKey || event.key !== "Backspace") {
        return;
      }
      if (selectedIds.length > 0 && !modalOpen && !isTextEntry(document.activeElement)) {
        event.preventDefault();
        setDialog({ kind: "delete", ids: selectedIds, bulk: true });
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [selectedIds, modalOpen]);

  const toggle = (id: number, on: boolean) =>
    setSelected((current) => {
      const next = new Set(current);
      if (on) {
        next.add(id);
      } else {
        next.delete(id);
      }
      return next;
    });

  const togglePin = (entry: DictionaryEntry) => {
    setDictionaryPinned(entry.id, !entry.pinned)
      .then(reload)
      .catch((err: unknown) => setError(errorMessage(err)));
  };

  const deleteIds = async (ids: number[]) => {
    await deleteDictionaryEntries(ids);
    setSelected((current) => new Set([...current].filter((id) => !ids.includes(id))));
    setDialog(null);
    await reload();
  };

  let modal = null;
  if (dialog?.kind === "edit") {
    modal = (
      <DictionaryDialog
        entry={dialog.entry}
        onSaved={() => {
          setDialog(null);
          void reload();
        }}
        onClose={() => setDialog(null)}
      />
    );
  } else if (dialog?.kind === "delete") {
    const { ids, bulk } = dialog;
    modal = (
      <ConfirmDialog
        title={
          bulk
            ? `Delete ${ids.length} ${ids.length === 1 ? "entry" : "entries"}?`
            : "Delete this word?"
        }
        message={bulk ? undefined : "It will be removed from your dictionary."}
        confirmLabel={bulk ? "Delete" : "Yes, delete it"}
        onConfirm={() => deleteIds(ids)}
        onCancel={() => setDialog(null)}
      />
    );
  } else if (importer.session) {
    modal = (
      <ImportDialog
        flow={importer}
        title="Import words"
        hint="One word per row, or misspelling,correction."
        importLabel={(count) => `Import ${count} ${count === 1 ? "item" : "items"}`}
        allExistText={(count) =>
          count === 1
            ? "This item already exists in your dictionary"
            : `All ${count} items already exist in your dictionary`
        }
      />
    );
  }

  const actions = (
    <>
      <button
        type="button"
        className="primary"
        onClick={() => setDialog({ kind: "edit", entry: null })}
      >
        Add word
      </button>
      <button type="button" onClick={importer.pick}>
        Import CSV
      </button>
      <ImportFileInput flow={importer} accept=".csv,text/csv" />
    </>
  );
  const pageError = importer.error ?? error;

  return (
    <Page title="Dictionary" actions={actions} modal={modal}>
      <input
        type="search"
        className="search"
        aria-label="Search dictionary"
        placeholder="Search"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
      />
      {selectedIds.length > 0 && (
        <div className="selection-bar">
          <span>{`${selectedIds.length} selected`}</span>
          <button
            type="button"
            className="danger"
            onClick={() => setDialog({ kind: "delete", ids: selectedIds, bulk: true })}
          >
            Delete
          </button>
        </div>
      )}
      {pageError && (
        <p className="error" role="alert">
          {pageError}
        </p>
      )}
      {entries === null ? (
        <p className="muted">Loading…</p>
      ) : entries.length === 0 ? (
        <p className="empty">
          No words yet. Add names, jargon and spellings Shrimp should get right.
        </p>
      ) : visible.length === 0 ? (
        <p className="empty">No matches.</p>
      ) : (
        <ul className="list">
          {visible.map((entry) => (
            <DictionaryRow
              key={entry.id}
              entry={entry}
              selected={selected.has(entry.id)}
              onSelect={(on) => toggle(entry.id, on)}
              onPin={() => togglePin(entry)}
              onEdit={() => setDialog({ kind: "edit", entry })}
              onDelete={() => setDialog({ kind: "delete", ids: [entry.id], bulk: false })}
            />
          ))}
        </ul>
      )}
    </Page>
  );
}
