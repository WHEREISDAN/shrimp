import { useEffect, useState } from "react";
import { Trans, useTranslation } from "react-i18next";
import { EVENTS, getAppInfo, getShortcuts } from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";
import { useInsights } from "./useInsights";

interface GreetingProps {
  onOpenInsights: () => void;
}

type Part = "morning" | "afternoon" | "evening";

/** Wispr Flow shows the push-to-talk shortcut only before this many dictated words. */
const SHORTCUT_HINT_WORDS = 2_000;

const partOfDay = (hour: number): Part =>
  hour < 12 ? "morning" : hour < 18 ? "afternoon" : "evening";

/**
 * Home's greeting, with the push-to-talk shortcut until 2,000 words are dictated,
 * and the week's stats card (STA-02).
 */
export function Greeting({ onOpenInsights }: GreetingProps) {
  const { t } = useTranslation();
  const [name, setName] = useState<string | null>(null);
  const [shortcut, setShortcut] = useState<string | null>(null);
  const { insights } = useInsights();
  const [part] = useState(() => partOfDay(new Date().getHours()));

  useEffect(() => {
    getAppInfo()
      .then((info) => setName(info.userName))
      .catch(() => undefined);
  }, []);
  const loadShortcut = () => {
    getShortcuts()
      .then((list) => {
        const row = list.rows.find((candidate) => candidate.action === "pushToTalk");
        setShortcut(row?.shortcuts[0] ?? null);
      })
      .catch(() => undefined);
  };
  useEffect(loadShortcut, []);
  useTauriEvent(EVENTS.settings, loadShortcut);

  const greeting = name ? t(`home.${part}`, { name }) : t(`home.${part}Anonymous`);
  const showShortcut =
    shortcut !== null && insights !== null && insights.totalWords < SHORTCUT_HINT_WORDS;
  return (
    <>
      <header className="greeting">
        <h1>{greeting}</h1>
        {showShortcut && (
          <p>
            <Trans
              i18nKey="home.holdToDictate"
              values={{ shortcut }}
              components={{ kbd: <kbd /> }}
            />
          </p>
        )}
      </header>
      {insights && (
        <button
          type="button"
          className="stats-card"
          aria-label={t("home.statsCard")}
          onClick={onOpenInsights}
        >
          <span className="stats-card-label">{t("home.thisWeek")}</span>
          <span className="stats-card-value">
            {t("insights.words", { count: insights.weekWords })}
          </span>
          <span className="stats-card-meta">
            {t("home.wpmShort", { count: insights.wpm })}
            {insights.currentStreak > 1 &&
              ` · ${t("home.streak", { count: insights.currentStreak })}`}
          </span>
          <span className="stats-card-link" aria-hidden="true">
            {t("home.viewInsights")} ›
          </span>
        </button>
      )}
    </>
  );
}
