import { useState, type ChangeEvent } from "react";
import { errorMessage, type SkippedRow } from "../lib/ipc";

const MAX_BYTES = 3 * 1024 * 1024;

/** What the backend would do with a file, reduced to what the dialog shows. */
export interface ImportPreview {
  added: string[];
  duplicates: number;
  skipped: SkippedRow[];
}

interface FileImportOptions {
  /** Lower-case, with the dot: ".csv". */
  extension: string;
  preview: (text: string) => Promise<ImportPreview>;
  commit: (text: string) => Promise<number>;
  onImported: () => void;
}

export interface FileImport {
  /** Ref callback for the hidden file input. */
  attachInput: (input: HTMLInputElement | null) => void;
  pick: () => void;
  onFileChange: (event: ChangeEvent<HTMLInputElement>) => void;
  /** Problems with the chosen file, shown on the page. */
  error: string | null;
  /** Set while the preview dialog is open. */
  session: { preview: ImportPreview } | null;
  /** A failed import, shown in the dialog. */
  importError: string | null;
  busy: boolean;
  confirm: () => void;
  close: () => void;
}

/** Picks a file, validates it, previews the import, and commits it on confirmation. */
export function useFileImport({
  extension,
  preview,
  commit,
  onImported,
}: FileImportOptions): FileImport {
  const [input, setInput] = useState<HTMLInputElement | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [session, setSession] = useState<{ text: string; preview: ImportPreview } | null>(null);
  const [importError, setImportError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const load = async (file: File) => {
    try {
      const text = await file.text();
      setSession({ text, preview: await preview(text) });
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  const onFileChange = (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) {
      return;
    }
    setError(null);
    if (!file.name.toLowerCase().endsWith(extension)) {
      setError(`Incorrect file type. Please upload a ${extension} file`);
    } else if (file.size > MAX_BYTES) {
      setError("File is too large (max 3MB)");
    } else {
      void load(file);
    }
  };

  const close = () => {
    setSession(null);
    setImportError(null);
    setBusy(false);
  };

  const confirm = () => {
    if (!session) {
      return;
    }
    setBusy(true);
    setImportError(null);
    commit(session.text)
      .then(() => {
        onImported();
        close();
      })
      .catch((err: unknown) => {
        setImportError(errorMessage(err));
        setBusy(false);
      });
  };

  return {
    attachInput: setInput,
    pick: () => input?.click(),
    onFileChange,
    error,
    session,
    importError,
    busy,
    confirm,
    close,
  };
}
