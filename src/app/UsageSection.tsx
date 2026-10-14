import { useTranslation } from "react-i18next";
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

interface UsageSectionProps {
  usage: UsageActivity[];
}

/** Words per usage category, with Power, Moderate or Light usage (STA-06). */
export function UsageSection({ usage }: UsageSectionProps) {
  const { t } = useTranslation();
  const total = usage.reduce((sum, row) => sum + row.words, 0);
  return (
    <section className="card stack" aria-labelledby="usage-heading">
      <h2 id="usage-heading">{t("insights.usage")}</h2>
      <p className="muted">{t("insights.usageHint")}</p>
      <ol className="app-bars usage-bars">
        {usage.map((row) => {
          const percent = total > 0 ? Math.round((row.words / total) * 100) : 0;
          const level = t(`insights.usageLevel.${row.level}`);
          const title = t("insights.usageTitle", {
            words: row.words,
            level,
            range: t(`insights.usageRange.${row.level}`),
          });
          return (
            <li key={row.category} title={title}>
              <span className="app-name">{t(`insights.usageCategory.${row.category}`)}</span>
              <span className="app-bar" aria-hidden="true">
                <span style={{ width: `${percent}%` }} />
              </span>
              <span className="muted">
                {t("insights.usageShare", { percent })} ·{" "}
                {t(COUNT_KEYS[row.category], { count: row.count })}
              </span>
              <span className={`usage-level usage-${row.level}`}>{level}</span>
            </li>
          );
        })}
      </ol>
    </section>
  );
}
