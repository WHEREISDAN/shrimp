import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  deleteDictionaryEntries,
  errorMessage,
  importDictionary,
  listDictionary,
  previewDictionaryImport,
  setDictionaryPinned,
  type DictionaryEntry,
} from "../lib/ipc";
import { AutoLearnToggle } from "./AutoLearnToggle";
import { ConfirmDialog } from "./ConfirmDialog";
import { DictionaryDialog } from "./DictionaryDialog";
import { DictionaryRow } from "./DictionaryRow";
import { ImportDialog, ImportFileInput } from "./ImportDialog";
import { Page } from "./Page";
import { useFileImport } from "./useFileImport";
import { primaryKey } from "../lib/platform";

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
  const { t } = useTranslation();
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
      if (!primaryKey(event) || event.key !== "Backspace") {
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
          bulk ? t("dictionary.deleteEntries", { count: ids.length }) : t("dictionary.deleteWord")
        }
        message={bulk ? undefined : t("dictionary.deleteWordBody")}
        confirmLabel={bulk ? t("common.delete") : t("dictionary.confirmDelete")}
        onConfirm={() => deleteIds(ids)}
        onCancel={() => setDialog(null)}
      />
    );
  } else if (importer.session) {
    modal = (
      <ImportDialog
        flow={importer}
        title={t("dictionary.importTitle")}
        hint={t("dictionary.importHint")}
        importLabel={(count) => t("dictionary.importItems", { count })}
        allExistText={(count) => t("dictionary.importAllExist", { count })}
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
        {t("dictionary.addWord")}
      </button>
      <button type="button" onClick={importer.pick}>
        {t("dictionary.importCsv")}
      </button>
      <ImportFileInput flow={importer} accept=".csv,text/csv" />
    </>
  );
  const pageError = importer.error ?? error;

  return (
    <Page title={t("dictionary.title")} actions={actions} modal={modal}>
      <AutoLearnToggle />
      <input
        type="search"
        className="search"
        aria-label={t("dictionary.search")}
        placeholder={t("dictionary.searchPlaceholder")}
        value={query}
        onChange={(event) => setQuery(event.target.value)}
      />
      {selectedIds.length > 0 && (
        <div className="selection-bar">
          <span>{t("dictionary.selected", { count: selectedIds.length })}</span>
          <button
            type="button"
            className="danger"
            onClick={() => setDialog({ kind: "delete", ids: selectedIds, bulk: true })}
          >
            {t("common.delete")}
          </button>
        </div>
      )}
      {pageError && (
        <p className="error" role="alert">
          {pageError}
        </p>
      )}
      {entries === null ? (
        <p className="muted">{t("common.loading")}</p>
      ) : entries.length === 0 ? (
        <p className="empty">{t("dictionary.empty")}</p>
      ) : visible.length === 0 ? (
        <p className="empty">{t("dictionary.noMatches")}</p>
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
