import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { errorMessage, EVENTS, getLanguages, type Languages } from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";
import { languageNames } from "./languageChoice";
import { LanguagesDialog } from "./LanguagesDialog";

interface LanguagesSectionProps {
  /** Increments each time the section should scroll into view. */
  reveal?: number;
}

/** Settings › Languages (LANG-01..05). */
export function LanguagesSection({ reveal = 0 }: LanguagesSectionProps) {
  const { t, i18n } = useTranslation();
  const [languages, setLanguages] = useState<Languages | null>(null);
  const [editing, setEditing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const headingRef = useRef<HTMLHeadingElement>(null);

  const load = useCallback(() => {
    getLanguages()
      .then(setLanguages)
      .catch((err: unknown) => setError(errorMessage(err)));
  }, []);
  useEffect(load, [load]);
  useTauriEvent(EVENTS.settings, load);

  useEffect(() => {
    if (reveal === 0) {
      return;
    }
    const frame = requestAnimationFrame(() => {
      headingRef.current?.scrollIntoView({ block: "start" });
      headingRef.current?.focus();
    });
    return () => cancelAnimationFrame(frame);
  }, [reveal]);

  const chosen = languages?.chosen;
  const summary: string = chosen
    ? chosen.autoDetect
      ? t("languages.autoDetect")
      : languageNames(chosen.selected, languages.options, i18n.language).join(", ")
    : t("common.loading");

  return (
    <section className="card stack" aria-labelledby="languages-heading" id="languages">
      <h2 id="languages-heading" ref={headingRef} tabIndex={-1}>
        {t("languages.title")}
      </h2>
      <div className="row">
        <span>{summary}</span>
        <button type="button" disabled={!languages} onClick={() => setEditing(true)}>
          {t("languages.change")}
        </button>
      </div>
      <p className="muted">{t("languages.body")}</p>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {editing && languages && (
        <LanguagesDialog
          options={languages.options}
          chosen={languages.chosen}
          onSaved={(saved) => setLanguages({ ...languages, chosen: saved })}
          onClose={() => setEditing(false)}
        />
      )}
    </section>
  );
}
