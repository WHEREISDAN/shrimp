import { Share, TriangleAlert } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Section } from "@/components/settings";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { Insights } from "../lib/ipc";
import { ActivityHeatmap } from "./ActivityHeatmap";
import { Page } from "./Page";
import { ShareDialog } from "./ShareDialog";
import { UsageBar, UsageSection } from "./UsageSection";
import { useInsights } from "./useInsights";
import { useSettings } from "./useSettings";
import { VoiceProfileCard } from "./VoiceProfileCard";

interface InsightsPageProps {
  /** Increments each time the voice profile should come into view. */
  revealVoiceProfile?: number;
}

/** Insights (STA-01, STA-03, STA-04, STA-06). */
export function InsightsPage({ revealVoiceProfile = 0 }: InsightsPageProps) {
  const { t } = useTranslation();
  const { insights, error, reload } = useInsights();
  const { settings } = useSettings();
  const [sharing, setSharing] = useState(false);

  return (
    <Page
      title={t("insights.title")}
      actions={
        <Button disabled={!insights} onClick={() => setSharing(true)}>
          <Share aria-hidden="true" />
          {t("insights.share")}
        </Button>
      }
      modal={
        sharing && insights && <ShareDialog insights={insights} onClose={() => setSharing(false)} />
      }
    >
      {error && (
        <div
          className="flex flex-wrap items-center gap-x-3 gap-y-2 rounded-lg border bg-muted/50 px-3.5 py-2.5 text-sm"
          role="alert"
        >
          <TriangleAlert aria-hidden="true" className="size-4 shrink-0 text-destructive" />
          <p className="min-w-0 flex-1 basis-48">{t("insights.loadError")}</p>
          <Button size="sm" onClick={reload}>
            {t("common.tryAgain")}
          </Button>
        </div>
      )}
      {!insights && !error && <p className="text-muted-foreground">{t("common.loading")}</p>}
      {insights && <Stats insights={insights} />}
      {settings && <VoiceProfileCard provider={settings.ai.provider} reveal={revealVoiceProfile} />}
    </Page>
  );
}

function Stats({ insights }: { insights: Insights }) {
  const { t } = useTranslation();
  const month = insights.month;
  const change = month ? Math.round(((month.current - month.previous) / month.previous) * 100) : 0;
  const most = Math.max(1, insights.apps[0]?.words ?? 1);
  return (
    <>
      <div className="grid grid-cols-[repeat(auto-fill,minmax(11rem,1fr))] gap-3">
        <Tile
          label={t("insights.wpm")}
          value={t("home.wpmShort", { count: insights.wpm })}
          hint={t("insights.wpmHint")}
        />
        <Tile
          label={t("insights.totalWords")}
          value={t("insights.words", { count: insights.totalWords })}
          hint={
            month
              ? t("insights.monthHint", { current: month.current, previous: month.previous })
              : undefined
          }
          badge={
            month
              ? change >= 0
                ? t("insights.monthUp", { percent: change })
                : t("insights.monthDown", { percent: -change })
              : undefined
          }
          badgeTone={change >= 0 ? "success" : "default"}
        />
        <Tile
          label={t("insights.fixes")}
          value={t("insights.words", {
            count: insights.wordsCorrected + insights.dictionaryFixes,
          })}
          hint={t("insights.fixesHint", {
            corrected: insights.wordsCorrected,
            dictionary: insights.dictionaryFixes,
          })}
        />
        <Tile
          label={t("insights.currentStreak")}
          value={t("insights.days", { count: insights.currentStreak })}
        />
        <Tile
          label={t("insights.longestStreak")}
          value={t("insights.days", { count: insights.longestStreak })}
        />
        <Tile
          label={t("insights.apps")}
          value={t("insights.appsUsed", { count: insights.appsUsed })}
          hint={insights.topApp ? t("insights.topAppHint", { app: insights.topApp }) : undefined}
        />
      </div>
      <Section headingId="activity-heading" title={t("insights.activity")}>
        <div className="rounded-xl border bg-card p-4 shadow-xs">
          <ActivityHeatmap days={insights.days} />
        </div>
      </Section>
      <Section headingId="apps-heading" title={t("insights.apps")}>
        {insights.apps.length === 0 ? (
          <p className="rounded-xl border border-dashed px-6 py-10 text-center text-muted-foreground">
            {t("insights.noApps")}
          </p>
        ) : (
          <ol className="m-0 flex list-none flex-col divide-y divide-separator rounded-xl border bg-card p-0 shadow-xs">
            {insights.apps.map((app) => (
              <li key={app.name} className="flex flex-col gap-2 px-4 py-3">
                <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-0.5">
                  <span className="app-name min-w-0 flex-1 basis-32 truncate font-medium">
                    {app.name}
                  </span>
                  <span className="text-sm text-muted-foreground tabular-nums">
                    {t("insights.appWords", { words: app.words, dictations: app.dictations })}
                  </span>
                </div>
                <UsageBar percent={(app.words / most) * 100} />
              </li>
            ))}
          </ol>
        )}
      </Section>
      <UsageSection usage={insights.usage} />
    </>
  );
}

interface TileProps {
  label: string;
  value: string;
  hint?: string | undefined;
  badge?: string | undefined;
  badgeTone?: "success" | "default";
}

function Tile({ label, value, hint, badge, badgeTone = "success" }: TileProps) {
  return (
    <div className="flex flex-col gap-1 rounded-xl border bg-card p-4 shadow-xs" title={hint}>
      <p className="text-sm font-medium text-muted-foreground">{label}</p>
      <p className="font-display text-2xl font-semibold tracking-[-0.01em] tabular-nums">{value}</p>
      {badge && (
        <Badge variant={badgeTone} className="mt-0.5">
          {badge}
        </Badge>
      )}
      {hint && <p className="mt-0.5 text-sm text-muted-foreground">{hint}</p>}
    </div>
  );
}
