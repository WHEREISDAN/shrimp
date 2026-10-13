import type { TFunction } from "i18next";
import type { HistoryItem } from "../lib/ipc";

export interface HistoryGroup {
  /** The local date, YYYY-MM-DD. */
  key: string;
  label: string;
  entries: HistoryItem[];
}

const dayKey = (date: Date) =>
  `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;

/** Entries (newest first) grouped by local day: Today, Yesterday, then dates. */
export function groupByDay(
  entries: HistoryItem[],
  t: TFunction,
  language: string,
  now: Date = new Date(),
): HistoryGroup[] {
  const today = dayKey(now);
  const yesterday = dayKey(new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1));
  const format = new Intl.DateTimeFormat(language, {
    weekday: "long",
    month: "long",
    day: "numeric",
  });
  const groups: HistoryGroup[] = [];
  for (const entry of entries) {
    const date = new Date(entry.createdAt);
    const key = dayKey(date);
    const last = groups[groups.length - 1];
    if (last?.key === key) {
      last.entries.push(entry);
      continue;
    }
    const label =
      key === today
        ? t("history.today")
        : key === yesterday
          ? t("history.yesterday")
          : format.format(date);
    groups.push({ key, label, entries: [entry] });
  }
  return groups;
}

/** "English" for "en", in the interface language. */
export function languageName(code: string | null, language: string): string | null {
  if (!code) {
    return null;
  }
  try {
    return new Intl.DisplayNames([language], { type: "language" }).of(code) ?? code;
  } catch {
    return code;
  }
}
