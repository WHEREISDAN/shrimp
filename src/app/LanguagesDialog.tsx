import { useState } from "react";
import { useTranslation } from "react-i18next";
import { errorMessage, setLanguages, type LanguageChoice, type LanguageOption } from "../lib/ipc";
import { Dialog } from "./Dialog";
import { languageName, toggleLanguage } from "./languageChoice";
import { ToggleRow } from "./ToggleRow";

interface LanguagesDialogProps {
  options: LanguageOption[];
  chosen: LanguageChoice;
  onSaved: (chosen: LanguageChoice) => void;
  onClose: () => void;
}

/** Settings › Languages (LANG-01, LANG-03): auto-detect or a searchable multi-select. */
export function LanguagesDialog({ options, chosen, onSaved, onClose }: LanguagesDialogProps) {
  const { t, i18n } = useTranslation();
  const [autoDetect, setAutoDetect] = useState(chosen.autoDetect);
  const [selected, setSelected] = useState(chosen.selected);
  const [query, setQuery] = useState("");
  const [error, setError] = useState<string | null>(null);

  const named = options.map((option) => ({
    ...option,
    shownName: languageName(option.id, options, i18n.language),
  }));
  const needle = query.trim().toLocaleLowerCase(i18n.language);
  const shown = needle
    ? named.filter((option) =>
        [option.shownName, option.name].some((name) =>
          name.toLocaleLowerCase(i18n.language).includes(needle),
        ),
      )
    : named;

  const save = () => {
    if (!autoDetect && selected.length === 0) {
      setError(t("languages.noneSelected"));
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
    <Dialog title={t("languages.title")} onClose={onClose} wide>
      <ToggleRow
        label={t("languages.autoDetect")}
        description={t("languages.autoDetectBody")}
        checked={autoDetect}
        onChange={setAutoDetect}
      />
      <input
        type="search"
        aria-label={t("languages.search")}
        placeholder={t("languages.search")}
        value={query}
        onChange={(event) => setQuery(event.target.value)}
      />
      <ul className="language-list" aria-label={t("languages.title")}>
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
              <span>{option.shownName}</span>
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
          {t("common.cancel")}
        </button>
        <button type="button" className="primary" onClick={save}>
          {t("dialog.save")}
        </button>
      </div>
    </Dialog>
  );
}
