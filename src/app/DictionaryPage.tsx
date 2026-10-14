import { Plus, Search, Trash2, Upload } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { NativeSelect } from "@/components/ui/native-select";
import {
  deleteDictionaryEntries,
  errorMessage,
  importDictionary,
  listDictionary,
  previewDictionaryImport,
  setDictionaryPinned,
  type DictionaryEntry,
  type DictionarySort,
} from "../lib/ipc";
import { AutoLearnToggle } from "./AutoLearnToggle";
import { ConfirmDialog } from "./ConfirmDialog";
import { DictionaryDialog } from "./DictionaryDialog";
import { DictionaryRow } from "./DictionaryRow";
import { ImportDialog, ImportFileInput } from "./ImportDialog";
import { Page } from "./Page";
import { useFileImport } from "./useFileImport";
import { useSettings } from "./useSettings";
import { primaryKey } from "../lib/platform";
import { DICTIONARY_SORTS, sortEntries } from "./dictionarySort";

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
  const { t, i18n } = useTranslation();
  const { settings, update } = useSettings();
  const sort = settings?.dictionary.sort ?? "starred";
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
    () =>
      sortEntries(
        (entries ?? []).filter((entry) => matches(entry, needle)),
        sort,
        i18n.language,
      ),
    [entries, needle, sort, i18n.language],
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
      <Button variant="default" onClick={() => setDialog({ kind: "edit", entry: null })}>
        <Plus aria-hidden="true" />
        {t("dictionary.addWord")}
      </Button>
      <Button onClick={importer.pick}>
        <Upload aria-hidden="true" />
        {t("dictionary.importCsv")}
      </Button>
      <ImportFileInput flow={importer} accept=".csv,text/csv" />
    </>
  );
  const pageError = importer.error ?? error;

  return (
    <Page title={t("dictionary.title")} actions={actions} modal={modal}>
      <AutoLearnToggle />
      <div className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
          <div className="relative min-w-48 flex-1">
            <Search
              aria-hidden="true"
              className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground"
            />
            <Input
              type="search"
              className="pl-8"
              aria-label={t("dictionary.search")}
              placeholder={t("dictionary.searchPlaceholder")}
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
          </div>
          <label className="flex shrink-0 items-center gap-2 text-sm whitespace-nowrap text-muted-foreground">
            <span>{t("dictionary.sortBy")}</span>
            <NativeSelect
              wrapperClassName="min-w-40"
              value={sort}
              onChange={(event) =>
                void update({ dictionary: { sort: event.target.value as DictionarySort } })
              }
            >
              {DICTIONARY_SORTS.map((option) => (
                <option key={option} value={option}>
                  {t(`dictionary.sorts.${option}`)}
                </option>
              ))}
            </NativeSelect>
          </label>
        </div>
        {selectedIds.length > 0 && (
          <div className="flex min-h-9 items-center justify-between gap-3 rounded-lg bg-muted py-1 pr-1 pl-3 text-sm">
            <span className="font-medium tabular-nums">
              {t("dictionary.selected", { count: selectedIds.length })}
            </span>
            <Button
              variant="destructive"
              size="sm"
              onClick={() => setDialog({ kind: "delete", ids: selectedIds, bulk: true })}
            >
              <Trash2 aria-hidden="true" />
              {t("common.delete")}
            </Button>
          </div>
        )}
        {pageError && (
          <p className="px-1 text-sm text-destructive" role="alert">
            {pageError}
          </p>
        )}
        {entries === null ? (
          <p className="px-1 text-muted-foreground">{t("common.loading")}</p>
        ) : entries.length === 0 ? (
          <p className="rounded-xl border border-dashed px-6 py-10 text-center text-muted-foreground">
            {t("dictionary.empty")}
          </p>
        ) : visible.length === 0 ? (
          <p className="rounded-xl border border-dashed px-6 py-10 text-center text-muted-foreground">
            {t("dictionary.noMatches")}
          </p>
        ) : (
          <ul className="list flex flex-col divide-y divide-separator rounded-xl border bg-card shadow-xs">
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
      </div>
    </Page>
  );
}
