import { useTranslation } from "react-i18next";
import type { SkippedRow } from "../lib/ipc";
import { Dialog } from "./Dialog";
import type { FileImport } from "./useFileImport";

const MAX_LISTED = 50;

interface ImportDialogProps {
  flow: FileImport;
  title: string;
  hint: string;
  /** "Import 3 items" */
  importLabel: (count: number) => string;
  /** "All 3 items already exist in your dictionary" */
  allExistText: (count: number) => string;
}

export function ImportFileInput({ flow, accept }: { flow: FileImport; accept: string }) {
  const { attachInput, onFileChange } = flow;
  return (
    <input
      ref={attachInput}
      type="file"
      accept={accept}
      hidden
      data-testid="import-file"
      onChange={onFileChange}
    />
  );
}

export function ImportDialog({ flow, title, hint, importLabel, allExistText }: ImportDialogProps) {
  const { t } = useTranslation();
  if (!flow.session) {
    return null;
  }
  const { added, duplicates, skipped } = flow.session.preview;
  const allExist = added.length === 0 && duplicates > 0;

  return (
    <Dialog title={title} onClose={flow.close}>
      <div className="import-preview">
        {allExist ? (
          <p>{allExistText(duplicates)}</p>
        ) : (
          <>
            {added.length === 0 && <p className="muted">{t("importFile.nothingNew")}</p>}
            <ul className="import-list">
              {added.slice(0, MAX_LISTED).map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ul>
            {added.length > MAX_LISTED && (
              <p className="muted">{t("importFile.more", { count: added.length - MAX_LISTED })}</p>
            )}
            {duplicates > 0 && (
              <p className="muted">{t("importFile.duplicates", { count: duplicates })}</p>
            )}
          </>
        )}
        <SkippedList rows={skipped} />
      </div>
      {flow.importError && (
        <p className="error" role="alert">
          {flow.importError}
        </p>
      )}
      <div className="dialog-actions">
        {added.length > 0 ? (
          <>
            <button type="button" onClick={flow.close}>
              {t("common.cancel")}
            </button>
            <button type="button" className="primary" onClick={flow.confirm} disabled={flow.busy}>
              {importLabel(added.length)}
            </button>
          </>
        ) : (
          <button type="button" className="primary" onClick={flow.close}>
            {allExist ? t("importFile.done") : t("common.close")}
          </button>
        )}
      </div>
      <p className="muted hint">{hint}</p>
    </Dialog>
  );
}

function SkippedList({ rows }: { rows: SkippedRow[] }) {
  const { t } = useTranslation();
  if (rows.length === 0) {
    return null;
  }
  return (
    <ul className="import-list skipped">
      {rows.map((row) => (
        <li key={`${row.line}-${row.reason}`} className="error">
          {t("importFile.skippedLine", { line: row.line, reason: row.reason })}
        </li>
      ))}
    </ul>
  );
}
