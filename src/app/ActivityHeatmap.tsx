import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { DayActivity } from "../lib/ipc";

const WEEKS = 53;
const CELL = 11;
const GAP = 3;

/** Wispr Flow's bands: 0, 1–249, 250–499, 500–749, 750+ words. */
export function band(words: number): number {
  if (words <= 0) {
    return 0;
  }
  return Math.min(4, Math.floor(words / 250) + 1);
}

const dayKey = (date: Date) =>
  `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;

interface ActivityHeatmapProps {
  days: DayActivity[];
  /** Today, for tests. */
  today?: Date;
}

/** A year of days, Sunday to Saturday in each column (STA-01). */
export function ActivityHeatmap({ days, today: fixedToday }: ActivityHeatmapProps) {
  const { t, i18n } = useTranslation();
  const [now] = useState(() => new Date());
  const today = fixedToday ?? now;
  const byDay = useMemo(() => new Map(days.map((day) => [day.day, day])), [days]);
  const format = new Intl.DateTimeFormat(i18n.language, { dateStyle: "medium" });
  // The last column ends with today; the first starts on a Sunday.
  const start = new Date(today.getFullYear(), today.getMonth(), today.getDate() - today.getDay());
  start.setDate(start.getDate() - (WEEKS - 1) * 7);
  const cells = [];
  for (let week = 0; week < WEEKS; week++) {
    for (let weekday = 0; weekday < 7; weekday++) {
      const date = new Date(
        start.getFullYear(),
        start.getMonth(),
        start.getDate() + week * 7 + weekday,
      );
      if (date > today) {
        continue;
      }
      const activity = byDay.get(dayKey(date));
      const label = activity
        ? t("insights.dayTitle", {
            date: format.format(date),
            words: activity.words,
            apps: activity.apps,
            app: activity.topApp ?? "—",
          })
        : t("insights.dayEmpty", { date: format.format(date) });
      cells.push(
        <rect
          key={`${week}-${weekday}`}
          className={`heat heat-${band(activity?.words ?? 0)}`}
          x={week * (CELL + GAP)}
          y={weekday * (CELL + GAP)}
          width={CELL}
          height={CELL}
          rx={2}
        >
          <title>{label}</title>
        </rect>,
      );
    }
  }
  const words = days.reduce((sum, day) => sum + day.words, 0);
  return (
    <figure className="heatmap">
      <svg
        role="img"
        aria-label={t("insights.heatmap", { days: days.length, words })}
        viewBox={`0 0 ${WEEKS * (CELL + GAP)} ${7 * (CELL + GAP)}`}
      >
        {cells}
      </svg>
      <figcaption className="heat-legend" aria-hidden="true">
        <span>{t("insights.less")}</span>
        {[0, 1, 2, 3, 4].map((level) => (
          <span key={level} className={`heat-swatch heat-${level}`} />
        ))}
        <span>{t("insights.more")}</span>
      </figcaption>
    </figure>
  );
}
