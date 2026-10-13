import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { languageName } from "../app/languageChoice";
import { EVENTS, getLanguages, setActiveLanguage, type Languages } from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";

/** With several languages, one click picks the language to dictate in, or Auto (LANG-05). */
export function LanguageSwitch() {
  const { t, i18n } = useTranslation();
  const [languages, setLanguages] = useState<Languages | null>(null);
  const load = useCallback(() => {
    getLanguages()
      .then(setLanguages)
      .catch(() => undefined);
  }, []);
  useEffect(load, [load]);
  useTauriEvent(EVENTS.settings, load);

  const chosen = languages?.chosen;
  if (!languages || !chosen || chosen.autoDetect || chosen.selected.length < 2) {
    return null;
  }
  const name = (id: string) => languageName(id, languages.options, i18n.language);
  const choose = (id: string | null) =>
    void setActiveLanguage(id)
      .then((saved) => setLanguages({ ...languages, chosen: saved }))
      .catch(() => undefined);

  return (
    <div className="language-switch" role="group" aria-label={t("languageSwitch.label")}>
      <button
        type="button"
        className="language-chip"
        aria-pressed={chosen.active === null}
        title={t("languageSwitch.detectAmong", { languages: chosen.selected.map(name).join(", ") })}
        onClick={() => choose(null)}
      >
        {t("languageSwitch.auto")}
      </button>
      {chosen.selected.map((id) => (
        <button
          key={id}
          type="button"
          className="language-chip"
          aria-pressed={chosen.active === id}
          aria-label={name(id)}
          title={name(id)}
          onClick={() => choose(id)}
        >
          {id.toUpperCase()}
        </button>
      ))}
    </div>
  );
}
