import { ChevronRight } from "lucide-react";
import { useEffect, useState } from "react";
import { Trans, useTranslation } from "react-i18next";
import { Kbd } from "@/components/ui/kbd";
import { EVENTS, getAppInfo, getShortcuts, type DayActivity } from "../lib/ipc";
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

const isoDay = (date: Date) =>
  `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;

/** The last seven days' words as waveform bars, today on the right. */
function WeekBars({ days, today }: { days: DayActivity[]; today: Date }) {
  const words = new Map(days.map((day) => [day.day, day.words]));
  const week = Array.from({ length: 7 }, (_, index) => {
    const date = new Date(today);
    date.setDate(date.getDate() - (6 - index));
    return words.get(isoDay(date)) ?? 0;
  });
  const most = Math.max(...week, 1);
  return (
    <span aria-hidden="true" className="flex h-9 items-center gap-[3px] max-[720px]:hidden">
      {week.map((count, index) => (
        <span
          key={index}
          className={
            count > 0 ? "w-[5px] rounded-full bg-brand" : "w-[5px] rounded-full bg-foreground/12"
          }
          style={{ height: `${Math.max(14, Math.round((count / most) * 100))}%` }}
        />
      ))}
    </span>
  );
}

/**
 * Home's greeting, with the push-to-talk shortcut until 2,000 words are dictated,
 * and the week's stats card (STA-02).
 */
export function Greeting({ onOpenInsights }: GreetingProps) {
  const { t } = useTranslation();
  const [name, setName] = useState<string | null>(null);
  const [shortcut, setShortcut] = useState<string | null>(null);
  const { insights } = useInsights();
  const [today] = useState(() => new Date());
  const part = partOfDay(today.getHours());

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
      <header data-tauri-drag-region="deep" className="greeting flex flex-col gap-1.5">
        <h1 className="font-display text-3xl font-bold tracking-[-0.015em]">{greeting}</h1>
        {showShortcut && (
          <p className="text-muted-foreground">
            <Trans
              i18nKey="home.holdToDictate"
              values={{ shortcut }}
              components={{ kbd: <Kbd className="mx-0.5" /> }}
            />
          </p>
        )}
      </header>
      {insights && (
        <button
          type="button"
          className="stats-card group grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-x-4 gap-y-0.5 rounded-xl border bg-card px-5 py-4 text-left shadow-xs transition-[background-color,box-shadow] duration-150 outline-none hover:bg-muted/50 hover:shadow-sm focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring/70 dark:hover:bg-white/[0.035]"
          aria-label={t("home.statsCard")}
          onClick={onOpenInsights}
        >
          <span className="stats-card-label text-sm font-medium text-muted-foreground">
            {t("home.thisWeek")}
          </span>
          <span className="stats-card-value font-display text-2xl font-semibold tracking-[-0.01em] tabular-nums">
            {t("insights.words", { count: insights.weekWords })}
          </span>
          <span className="stats-card-meta text-sm text-muted-foreground tabular-nums">
            {t("home.wpmShort", { count: insights.wpm })}
            {insights.currentStreak > 1 &&
              ` · ${t("home.streak", { count: insights.currentStreak })}`}
          </span>
          <span className="col-start-2 row-span-3 row-start-1 pr-2">
            <WeekBars days={insights.days} today={today} />
          </span>
          <span
            className="stats-card-link col-start-3 row-span-3 row-start-1 flex items-center gap-0.5 text-sm font-medium text-brand-strong"
            aria-hidden="true"
          >
            {t("home.viewInsights")}
            <ChevronRight className="size-3.5 transition-transform duration-200 ease-spring group-hover:translate-x-0.5" />
          </span>
        </button>
      )}
    </>
  );
}
