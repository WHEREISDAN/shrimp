import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { Insights } from "../lib/ipc";
import { ActivityHeatmap } from "./ActivityHeatmap";
import { Page } from "./Page";
import { ShareDialog } from "./ShareDialog";
import { UsageSection } from "./UsageSection";
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
        <button type="button" disabled={!insights} onClick={() => setSharing(true)}>
          {t("insights.share")}
        </button>
      }
      modal={
        sharing && insights && <ShareDialog insights={insights} onClose={() => setSharing(false)} />
      }
    >
      {error && (
        <div className="banner" role="alert">
          <p>{t("insights.loadError")}</p>
          <button type="button" onClick={reload}>
            {t("common.tryAgain")}
          </button>
        </div>
      )}
      {!insights && !error && <p className="muted">{t("common.loading")}</p>}
      {insights && <Stats insights={insights} />}
      {settings && <VoiceProfileCard provider={settings.ai.provider} reveal={revealVoiceProfile} />}
    </Page>
  );
}

function Stats({ insights }: { insights: Insights }) {
  const { t } = useTranslation();
  const month = insights.month;
  const change = month ? Math.round(((month.current - month.previous) / month.previous) * 100) : 0;
  return (
    <>
      <div className="tiles">
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
      <section className="card stack" aria-labelledby="activity-heading">
        <h2 id="activity-heading">{t("insights.activity")}</h2>
        <ActivityHeatmap days={insights.days} />
      </section>
      <section className="card stack" aria-labelledby="apps-heading">
        <h2 id="apps-heading">{t("insights.apps")}</h2>
        {insights.apps.length === 0 ? (
          <p className="muted">{t("insights.noApps")}</p>
        ) : (
          <ol className="app-bars">
            {insights.apps.map((app) => (
              <li key={app.name}>
                <span className="app-name">{app.name}</span>
                <span className="app-bar" aria-hidden="true">
                  <span
                    style={{
                      width: `${(app.words / Math.max(1, insights.apps[0]?.words ?? 1)) * 100}%`,
                    }}
                  />
                </span>
                <span className="muted">
                  {t("insights.appWords", { words: app.words, dictations: app.dictations })}
                </span>
              </li>
            ))}
          </ol>
        )}
      </section>
      <UsageSection usage={insights.usage} />
    </>
  );
}

interface TileProps {
  label: string;
  value: string;
  hint?: string | undefined;
  badge?: string | undefined;
}

function Tile({ label, value, hint, badge }: TileProps) {
  return (
    <div className="tile" title={hint}>
      <p className="tile-label">{label}</p>
      <p className="tile-value">{value}</p>
      {badge && <p className="tile-badge">{badge}</p>}
      {hint && <p className="tile-hint">{hint}</p>}
    </div>
  );
}
