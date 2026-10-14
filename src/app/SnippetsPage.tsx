import { Info, Plus, Upload } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  deleteSnippets,
  errorMessage,
  importSnippets,
  listSnippets,
  previewSnippetImport,
  type Snippet,
  type SnippetSaved,
} from "../lib/ipc";
import { ConfirmDialog } from "./ConfirmDialog";
import { ImportDialog, ImportFileInput } from "./ImportDialog";
import { plainPreview } from "./markdownEdit";
import { Page } from "./Page";
import { SnippetDialog } from "./SnippetDialog";
import { useFileImport } from "./useFileImport";

type Dialog = { kind: "edit"; snippet: Snippet | null } | { kind: "delete"; snippet: Snippet };

const WARNING_MS = 10_000;

export function SnippetsPage() {
  const { t } = useTranslation();
  const [snippets, setSnippets] = useState<Snippet[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [warning, setWarning] = useState<string | null>(null);
  const [dialog, setDialog] = useState<Dialog | null>(null);

  const reload = useCallback(
    () =>
      listSnippets()
        .then(setSnippets)
        .catch((err: unknown) => setError(errorMessage(err))),
    [],
  );
  useEffect(() => {
    void reload();
  }, [reload]);

  useEffect(() => {
    if (warning === null) {
      return;
    }
    const timer = setTimeout(() => setWarning(null), WARNING_MS);
    return () => clearTimeout(timer);
  }, [warning]);

  const importer = useFileImport({
    extension: ".json",
    preview: async (json) => {
      const { added, duplicates, skipped } = await previewSnippetImport(json);
      return { added: added.map((item) => item.trigger), duplicates: duplicates.length, skipped };
    },
    commit: importSnippets,
    onImported: () => void reload(),
  });

  const saved = ({ warning: note }: SnippetSaved) => {
    setDialog(null);
    setWarning(note);
    void reload();
  };

  const remove = async (snippet: Snippet) => {
    await deleteSnippets([snippet.id]);
    setDialog(null);
    await reload();
  };

  let modal = null;
  if (dialog?.kind === "edit") {
    modal = (
      <SnippetDialog snippet={dialog.snippet} onSaved={saved} onClose={() => setDialog(null)} />
    );
  } else if (dialog?.kind === "delete") {
    const { snippet } = dialog;
    modal = (
      <ConfirmDialog
        title={t("snippets.deleteTitle")}
        message={t("snippets.deleteBody")}
        confirmLabel={t("snippets.confirmDelete")}
        onConfirm={() => remove(snippet)}
        onCancel={() => setDialog(null)}
      />
    );
  } else if (importer.session) {
    modal = (
      <ImportDialog
        flow={importer}
        title={t("snippets.importTitle")}
        hint={t("snippets.importHint")}
        importLabel={(count) => t("snippets.importItems", { count })}
        allExistText={(count) => t("snippets.importAllExist", { count })}
      />
    );
  }

  const actions = (
    <>
      <Button variant="default" onClick={() => setDialog({ kind: "edit", snippet: null })}>
        <Plus aria-hidden="true" />
        {t("snippets.add")}
      </Button>
      <Button onClick={importer.pick}>
        <Upload aria-hidden="true" />
        {t("snippets.importJson")}
      </Button>
      <ImportFileInput flow={importer} accept=".json,application/json" />
    </>
  );
  const pageError = importer.error ?? error;

  return (
    <Page title={t("snippets.title")} actions={actions} modal={modal}>
      <div className="flex flex-col gap-3">
        {warning && (
          <p
            className="flex items-start gap-2 rounded-lg border bg-muted/50 px-3.5 py-2.5 text-sm"
            role="status"
          >
            <Info aria-hidden="true" className="mt-px size-3.5 shrink-0 text-muted-foreground" />
            {warning}
          </p>
        )}
        {pageError && (
          <p className="px-1 text-sm text-destructive" role="alert">
            {pageError}
          </p>
        )}
        {snippets === null ? (
          <p className="px-1 text-muted-foreground">{t("common.loading")}</p>
        ) : snippets.length === 0 ? (
          <p className="rounded-xl border border-dashed px-6 py-10 text-center text-muted-foreground">
            {t("snippets.empty")}
          </p>
        ) : (
          <ul className="list flex flex-col divide-y divide-separator rounded-xl border bg-card shadow-xs">
            {snippets.map((snippet) => (
              <li
                key={snippet.id}
                className="list-row group/row flex min-h-(--row-min-h) items-center gap-3 py-2 pr-2 pl-4 transition-colors duration-100 first:rounded-t-xl last:rounded-b-xl hover:bg-muted/40 dark:hover:bg-white/[0.025]"
              >
                <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                  <p className="list-title font-medium [overflow-wrap:anywhere]">
                    {snippet.trigger}
                  </p>
                  <p className="truncate text-sm text-muted-foreground">
                    {plainPreview(snippet.expansion)}
                  </p>
                </div>
                <div className="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity duration-150 group-focus-within/row:opacity-100 group-hover/row:opacity-100">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setDialog({ kind: "edit", snippet })}
                  >
                    {t("snippets.edit")}
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="text-destructive hover:bg-destructive/10"
                    onClick={() => setDialog({ kind: "delete", snippet })}
                  >
                    {t("common.delete")}
                  </Button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </Page>
  );
}
