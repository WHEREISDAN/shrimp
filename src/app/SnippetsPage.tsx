import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
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
      <button
        type="button"
        className="primary"
        onClick={() => setDialog({ kind: "edit", snippet: null })}
      >
        {t("snippets.add")}
      </button>
      <button type="button" onClick={importer.pick}>
        {t("snippets.importJson")}
      </button>
      <ImportFileInput flow={importer} accept=".json,application/json" />
    </>
  );
  const pageError = importer.error ?? error;

  return (
    <Page title={t("snippets.title")} actions={actions} modal={modal}>
      {warning && (
        <p className="notice" role="status">
          {warning}
        </p>
      )}
      {pageError && (
        <p className="error" role="alert">
          {pageError}
        </p>
      )}
      {snippets === null ? (
        <p className="muted">{t("common.loading")}</p>
      ) : snippets.length === 0 ? (
        <p className="empty">{t("snippets.empty")}</p>
      ) : (
        <ul className="list">
          {snippets.map((snippet) => (
            <li key={snippet.id} className="list-row">
              <div className="list-main">
                <p className="list-title">{snippet.trigger}</p>
                <p className="list-preview muted">{plainPreview(snippet.expansion)}</p>
              </div>
              <div className="row-actions">
                <button type="button" onClick={() => setDialog({ kind: "edit", snippet })}>
                  {t("snippets.edit")}
                </button>
                <button type="button" onClick={() => setDialog({ kind: "delete", snippet })}>
                  {t("common.delete")}
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </Page>
  );
}
