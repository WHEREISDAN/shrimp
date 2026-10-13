import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { EVENTS, getShortcuts, type ShortcutList } from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";

interface ShortcutsSectionProps {
  onOpenShortcuts: () => void;
}

/** Settings › Shortcuts: what push to talk is, and the way into the Shortcuts dialog. */
export function ShortcutsSection({ onOpenShortcuts }: ShortcutsSectionProps) {
  const { t } = useTranslation();
  const [list, setList] = useState<ShortcutList | null>(null);
  const load = useCallback(() => {
    getShortcuts()
      .then(setList)
      .catch(() => undefined);
  }, []);
  useEffect(load, [load]);
  useTauriEvent(EVENTS.settings, load);
  const talk = list?.rows.find((row) => row.action === "pushToTalk");

  return (
    <section className="card stack" aria-labelledby="shortcut-settings-heading">
      <h2 id="shortcut-settings-heading">{t("shortcutsSection.title")}</h2>
      <div className="row">
        <span>
          {t("shortcutsSection.pushToTalk")} {talk && <kbd>{talk.shortcuts.join(" / ")}</kbd>}
        </span>
        <button type="button" onClick={onOpenShortcuts}>
          {t("shortcutsSection.change")}
        </button>
      </div>
      <p className="muted">{t("shortcutsSection.note")}</p>
    </section>
  );
}
