import { useState } from "react";
import { errorMessage, setLanguages, type LanguageChoice, type LanguageOption } from "../lib/ipc";
import { Dialog } from "./Dialog";
import { toggleLanguage } from "./languageChoice";
import { ToggleRow } from "./ToggleRow";

const NONE_SELECTED = "Please select at least one language or turn on auto-detect";

interface LanguagesDialogProps {
  options: LanguageOption[];
  chosen: LanguageChoice;
  onSaved: (chosen: LanguageChoice) => void;
  onClose: () => void;
}

/** Settings › Languages (LANG-01, LANG-03): auto-detect or a searchable multi-select. */
export function LanguagesDialog({ options, chosen, onSaved, onClose }: LanguagesDialogProps) {
  const [autoDetect, setAutoDetect] = useState(chosen.autoDetect);
  const [selected, setSelected] = useState(chosen.selected);
  const [query, setQuery] = useState("");
  const [error, setError] = useState<string | null>(null);

  const needle = query.trim().toLowerCase();
  const shown = needle
    ? options.filter((option) => option.name.toLowerCase().includes(needle))
    : options;

  const save = () => {
    if (!autoDetect && selected.length === 0) {
      setError(NONE_SELECTED);
      return;
    }
    setLanguages(autoDetect, selected)
      .then((saved) => {
        onSaved(saved);
        onClose();
      })
      .catch((err: unknown) => setError(errorMessage(err)));
  };

  return (
    <Dialog title="Languages" onClose={onClose} wide>
      <ToggleRow
        label="Auto-detect"
        description="Transcribe any language you speak."
        checked={autoDetect}
        onChange={setAutoDetect}
      />
      <input
        type="search"
        aria-label="Search languages"
        placeholder="Search languages"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
      />
      <ul className="language-list" aria-label="Languages">
        {shown.map((option) => (
          <li key={option.id}>
            <label className="check">
              <input
                type="checkbox"
                checked={selected.includes(option.id)}
                onChange={() => {
                  setError(null);
                  setSelected(toggleLanguage(selected, option.id, options));
                }}
              />
              <span>{option.name}</span>
            </label>
          </li>
        ))}
      </ul>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <div className="dialog-actions">
        <button type="button" onClick={onClose}>
          Cancel
        </button>
        <button type="button" className="primary" onClick={save}>
          Save
        </button>
      </div>
    </Dialog>
  );
}
