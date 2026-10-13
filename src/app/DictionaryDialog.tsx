import { useId, useState, type FormEvent } from "react";
import {
  addDictionaryEntry,
  errorMessage,
  updateDictionaryEntry,
  type DictionaryEntry,
} from "../lib/ipc";
import { Dialog } from "./Dialog";

interface DictionaryDialogProps {
  /** The entry being edited; null adds a new one. */
  entry: DictionaryEntry | null;
  onSaved: () => void;
  onClose: () => void;
}

export function DictionaryDialog({ entry, onSaved, onClose }: DictionaryDialogProps) {
  const [term, setTerm] = useState(entry?.term ?? "");
  const [isCorrection, setIsCorrection] = useState(Boolean(entry?.misspelling));
  const [misspelling, setMisspelling] = useState(entry?.misspelling ?? "");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const misspellingId = useId();

  const save = async (event: FormEvent) => {
    event.preventDefault();
    const input = { term, misspelling: isCorrection ? misspelling : null };
    setBusy(true);
    setError(null);
    try {
      await (entry ? updateDictionaryEntry(entry.id, input) : addDictionaryEntry(input));
      onSaved();
    } catch (err) {
      setError(errorMessage(err));
      setBusy(false);
    }
  };

  return (
    <Dialog title={entry ? "Edit word" : "Add word"} onClose={onClose}>
      <form className="form" onSubmit={(event) => void save(event)}>
        <label className="field">
          <span>Word</span>
          <input
            type="text"
            value={term}
            maxLength={60}
            autoComplete="off"
            autoFocus
            onChange={(event) => setTerm(event.target.value)}
          />
        </label>
        <label className="check">
          <input
            type="checkbox"
            checked={isCorrection}
            onChange={(event) => setIsCorrection(event.target.checked)}
          />
          <span>Correct a misspelling</span>
        </label>
        {isCorrection && (
          <div className="field">
            <label htmlFor={misspellingId}>Misspelling</label>
            <input
              id={misspellingId}
              type="text"
              value={misspelling}
              maxLength={60}
              autoComplete="off"
              aria-describedby={`${misspellingId}-hint`}
              onChange={(event) => setMisspelling(event.target.value)}
            />
            <p id={`${misspellingId}-hint`} className="muted hint">
              What Shrimp hears, replaced by the word above.
            </p>
          </div>
        )}
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        <div className="dialog-actions">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" className="primary" disabled={busy}>
            Save
          </button>
        </div>
      </form>
    </Dialog>
  );
}
