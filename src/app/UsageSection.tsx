import { useTranslation } from "react-i18next";
import { Note, Section } from "@/components/settings";
import { Badge } from "@/components/ui/badge";
import type { UsageActivity, UsageCategory } from "../lib/ipc";

/** What each row counts: messages, emails, prompts, documents or tasks. */
const COUNT_KEYS = {
  personalMessages: "insights.usageMessages",
  workMessages: "insights.usageMessages",
  emails: "insights.usageEmails",
  aiPrompts: "insights.usagePrompts",
  documents: "insights.usageDocuments",
  otherTasks: "insights.usageTasks",
} as const satisfies Record<UsageCategory, string>;

/** A horizontal bar: `percent` of its track filled in the brand colour. */
export function UsageBar({ percent }: { percent: number }) {
  return (
    <span className="block h-1.5 overflow-hidden rounded-full bg-muted" aria-hidden="true">
      <span
        className="block h-full rounded-full bg-brand transition-[width] duration-300 ease-out-quint"
        style={{ width: `${percent}%` }}
      />
    </span>
  );
}

interface UsageSectionProps {
  usage: UsageActivity[];
}

/** Words per usage category, with Power, Moderate or Light usage (STA-06). */
export function UsageSection({ usage }: UsageSectionProps) {
  const { t } = useTranslation();
  const total = usage.reduce((sum, row) => sum + row.words, 0);
  return (
    <Section headingId="usage-heading" title={t("insights.usage")}>
      <ol className="m-0 flex list-none flex-col divide-y divide-separator rounded-xl border bg-card p-0 shadow-xs">
        {usage.map((row) => {
          const percent = total > 0 ? Math.round((row.words / total) * 100) : 0;
          const level = t(`insights.usageLevel.${row.level}`);
          const title = t("insights.usageTitle", {
            words: row.words,
            level,
            range: t(`insights.usageRange.${row.level}`),
          });
          return (
            <li key={row.category} title={title} className="flex flex-col gap-2 px-4 py-3">
              <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-1">
                <span className="flex min-w-0 flex-1 basis-40 flex-wrap items-center gap-x-2 gap-y-1">
                  <span className="app-name font-medium">
                    {t(`insights.usageCategory.${row.category}`)}
                  </span>
                  <Badge variant={row.level === "power" ? "brand" : "default"}>{level}</Badge>
                </span>
                <span className="text-sm text-muted-foreground tabular-nums">
                  {t("insights.usageShare", { percent })} ·{" "}
                  {t(COUNT_KEYS[row.category], { count: row.count })}
                </span>
              </div>
              <UsageBar percent={percent} />
            </li>
          );
        })}
      </ol>
      <Note>{t("insights.usageHint")}</Note>
    </Section>
  );
}
