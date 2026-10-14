import { TriangleAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import type { SkippedRow } from "../lib/ipc";
import { Dialog, DialogFooter } from "./Dialog";
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
      <div className="flex max-h-64 flex-col gap-2 overflow-y-auto">
        {allExist ? (
          <p>{allExistText(duplicates)}</p>
        ) : (
          <>
            {added.length === 0 && (
              <p className="text-muted-foreground">{t("importFile.nothingNew")}</p>
            )}
            <ul className="flex flex-col divide-y divide-separator rounded-lg border bg-card empty:hidden">
              {added.slice(0, MAX_LISTED).map((item) => (
                <li key={item} className="px-3 py-1.5 [overflow-wrap:anywhere]">
                  {item}
                </li>
              ))}
            </ul>
            {added.length > MAX_LISTED && (
              <p className="text-sm text-muted-foreground">
                {t("importFile.more", { count: added.length - MAX_LISTED })}
              </p>
            )}
            {duplicates > 0 && (
              <p className="text-sm text-muted-foreground">
                {t("importFile.duplicates", { count: duplicates })}
              </p>
            )}
          </>
        )}
        <SkippedList rows={skipped} />
      </div>
      <p className="text-sm text-muted-foreground">{hint}</p>
      {flow.importError && (
        <p className="text-sm text-destructive" role="alert">
          {flow.importError}
        </p>
      )}
      <DialogFooter>
        {added.length > 0 ? (
          <>
            <Button onClick={flow.close}>{t("common.cancel")}</Button>
            <Button variant="default" onClick={flow.confirm} disabled={flow.busy}>
              {importLabel(added.length)}
            </Button>
          </>
        ) : (
          <Button variant="default" onClick={flow.close}>
            {allExist ? t("importFile.done") : t("common.close")}
          </Button>
        )}
      </DialogFooter>
    </Dialog>
  );
}

function SkippedList({ rows }: { rows: SkippedRow[] }) {
  const { t } = useTranslation();
  if (rows.length === 0) {
    return null;
  }
  return (
    <ul className="flex flex-col gap-1 text-sm text-destructive">
      {rows.map((row) => (
        <li key={`${row.line}-${row.reason}`} className="flex items-start gap-1.5">
          <TriangleAlert aria-hidden="true" className="mt-px size-3.5 shrink-0" />
          <span>{t("importFile.skippedLine", { line: row.line, reason: row.reason })}</span>
        </li>
      ))}
    </ul>
  );
}
