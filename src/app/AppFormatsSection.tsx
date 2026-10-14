import { useEffect, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { getFormatApps, type AppFormat, type KnownApp, type TranscriptFormat } from "../lib/ipc";

const FORMATS: TranscriptFormat[] = ["automatic", "unformatted", "properNouns", "lowercase"];

interface AppFormatsSectionProps {
  formats: AppFormat[];
  /** Saves the whole list. */
  onChange: (formats: AppFormat[]) => void;
}

/** Style › Formatting per app (FMT-15). */
export function AppFormatsSection({ formats, onChange }: AppFormatsSectionProps) {
  const { t } = useTranslation();
  const ids = useId();
  const [known, setKnown] = useState<KnownApp[]>([]);
  const [picked, setPicked] = useState("");
  const [format, setFormat] = useState<TranscriptFormat>("unformatted");

  useEffect(() => {
    let cancelled = false;
    getFormatApps()
      .then((apps) => !cancelled && setKnown(apps))
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);

  const listed = (id: string) =>
    formats.some((entry) => entry.appId.toLowerCase() === id.toLowerCase());
  const available = known.filter((app) => !listed(app.id));
  const choice = available.find((app) => app.id === picked) ?? available[0];
  const formatOptions = FORMATS.map((value) => (
    <option key={value} value={value}>
      {t(`style.appFormats.formats.${value}`)}
    </option>
  ));

  const add = () => {
    if (choice) {
      onChange([...formats, { appId: choice.id, appName: choice.name, format }]);
    }
  };
  const change = (appId: string, next: TranscriptFormat) =>
    onChange(formats.map((entry) => (entry.appId === appId ? { ...entry, format: next } : entry)));
  const remove = (appId: string) => onChange(formats.filter((entry) => entry.appId !== appId));

  return (
    <section className="card stack" aria-labelledby={`${ids}-heading`}>
      <h2 id={`${ids}-heading`}>{t("style.appFormats.title")}</h2>
      <p className="muted">{t("style.appFormats.body")}</p>
      {formats.length > 0 && (
        <ul className="list" aria-labelledby={`${ids}-heading`}>
          {formats.map((entry) => {
            const app = entry.appName || entry.appId;
            return (
              <li key={entry.appId} className="list-row">
                <div className="list-main">
                  <div className="list-title">{app}</div>
                  <div className="list-meta muted">{entry.appId}</div>
                </div>
                <select
                  aria-label={t("style.appFormats.formatFor", { app })}
                  value={entry.format}
                  onChange={(event) => change(entry.appId, event.target.value as TranscriptFormat)}
                >
                  {formatOptions}
                </select>
                <div className="row-actions">
                  <button
                    type="button"
                    aria-label={t("style.appFormats.removeApp", { app })}
                    onClick={() => remove(entry.appId)}
                  >
                    {t("style.appFormats.remove")}
                  </button>
                </div>
              </li>
            );
          })}
        </ul>
      )}
      {choice ? (
        <div className="inline-row">
          <label className="select-field">
            <span>{t("style.appFormats.app")}</span>
            <select value={choice.id} onChange={(event) => setPicked(event.target.value)}>
              {available.map((app) => (
                <option key={app.id} value={app.id}>
                  {app.name}
                </option>
              ))}
            </select>
          </label>
          <label className="select-field">
            <span>{t("style.appFormats.format")}</span>
            <select
              value={format}
              onChange={(event) => setFormat(event.target.value as TranscriptFormat)}
            >
              {formatOptions}
            </select>
          </label>
          <button type="button" onClick={add}>
            {t("style.appFormats.add")}
          </button>
        </div>
      ) : (
        known.length === 0 && <p className="muted">{t("style.appFormats.noApps")}</p>
      )}
      <p className="muted footnote">{t("style.appFormats.footnote")}</p>
    </section>
  );
}
