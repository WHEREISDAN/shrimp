import { useId, useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  addDictionaryEntry,
  errorMessage,
  updateDictionaryEntry,
  type DictionaryEntry,
} from "../lib/ipc";
import { Dialog, DialogFooter } from "./Dialog";

interface DictionaryDialogProps {
  /** The entry being edited; null adds a new one. */
  entry: DictionaryEntry | null;
  onSaved: () => void;
  onClose: () => void;
}

export function DictionaryDialog({ entry, onSaved, onClose }: DictionaryDialogProps) {
  const { t } = useTranslation();
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
    <Dialog title={entry ? t("dictionary.editWord") : t("dictionary.addWord")} onClose={onClose}>
      <form className="flex flex-col gap-4" onSubmit={(event) => void save(event)}>
        <label className="flex flex-col gap-1.5">
          <span className="text-sm font-medium">{t("dictionary.word")}</span>
          <Input
            type="text"
            value={term}
            maxLength={60}
            autoComplete="off"
            autoFocus
            onChange={(event) => setTerm(event.target.value)}
          />
        </label>
        <label className="flex items-start gap-3">
          <input
            type="checkbox"
            className="mt-0.5"
            checked={isCorrection}
            onChange={(event) => setIsCorrection(event.target.checked)}
          />
          <span>{t("dictionary.correctMisspelling")}</span>
        </label>
        {isCorrection && (
          <div className="flex flex-col gap-1.5">
            <label htmlFor={misspellingId} className="text-sm font-medium">
              {t("dictionary.misspelling")}
            </label>
            <Input
              id={misspellingId}
              type="text"
              value={misspelling}
              maxLength={60}
              autoComplete="off"
              aria-describedby={`${misspellingId}-hint`}
              onChange={(event) => setMisspelling(event.target.value)}
            />
            <p id={`${misspellingId}-hint`} className="text-sm text-muted-foreground">
              {t("dictionary.misspellingHint")}
            </p>
          </div>
        )}
        {error && (
          <p className="text-sm text-destructive" role="alert">
            {error}
          </p>
        )}
        <DialogFooter>
          <Button onClick={onClose}>{t("common.cancel")}</Button>
          <Button type="submit" variant="default" disabled={busy}>
            {t("dictionary.save")}
          </Button>
        </DialogFooter>
      </form>
    </Dialog>
  );
}
