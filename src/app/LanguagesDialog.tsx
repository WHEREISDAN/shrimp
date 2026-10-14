import { Search } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Group } from "@/components/settings";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { errorMessage, setLanguages, type LanguageChoice, type LanguageOption } from "../lib/ipc";
import { Dialog, DialogFooter } from "./Dialog";
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
      <Group>
        <ToggleRow
          label={t("languages.autoDetect")}
          description={t("languages.autoDetectBody")}
          checked={autoDetect}
          onChange={setAutoDetect}
        />
      </Group>
      <div className="flex flex-col gap-2">
        <div className="relative">
          <Search
            aria-hidden="true"
            className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground"
          />
          <Input
            type="search"
            className="pl-8"
            aria-label={t("languages.search")}
            placeholder={t("languages.search")}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
        </div>
        <ul
          className="flex max-h-64 flex-col divide-y divide-separator overflow-y-auto rounded-xl border bg-card shadow-xs empty:hidden"
          aria-label={t("languages.title")}
        >
          {shown.map((option) => (
            <li key={option.id}>
              <label className="flex items-start gap-3 px-4 py-2 transition-colors duration-100 hover:bg-muted/40 dark:hover:bg-white/[0.025]">
                <input
                  type="checkbox"
                  className="mt-0.5"
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
      </div>
      {error && (
        <p className="text-sm text-destructive" role="alert">
          {error}
        </p>
      )}
      <DialogFooter>
        <Button onClick={onClose}>{t("common.cancel")}</Button>
        <Button variant="default" onClick={save}>
          {t("dialog.save")}
        </Button>
      </DialogFooter>
    </Dialog>
  );
}
