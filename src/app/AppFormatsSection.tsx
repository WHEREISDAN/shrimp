import { useCallback, useEffect, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { Group, Note, Row, Section } from "@/components/settings";
import { Button } from "@/components/ui/button";
import { NativeSelect } from "@/components/ui/native-select";
import {
  EVENTS,
  getFormatApps,
  type AppFormat,
  type KnownApp,
  type TranscriptFormat,
} from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";

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

  const load = useCallback(() => {
    getFormatApps()
      .then(setKnown)
      .catch(() => undefined);
  }, []);
  useEffect(load, [load]);
  // The window may have been opened from another app, which the list now offers first.
  useTauriEvent<boolean>(EVENTS.mainVisible, (shown) => {
    if (shown) {
      load();
    }
  });

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
    <Section headingId={`${ids}-heading`} title={t("style.appFormats.title")}>
      <Note>{t("style.appFormats.body")}</Note>
      <Group>
        {formats.length > 0 && (
          <ul
            className="list flex flex-col divide-y divide-separator"
            aria-labelledby={`${ids}-heading`}
          >
            {formats.map((entry) => {
              const app = entry.appName || entry.appId;
              return (
                <li
                  key={entry.appId}
                  className="list-row flex min-h-(--row-min-h) flex-wrap items-center gap-x-3 gap-y-2 px-4 py-2.5"
                >
                  <div className="flex min-w-0 flex-1 basis-40 flex-col gap-0.5">
                    <div className="list-title font-medium">{app}</div>
                    <div className="text-sm [overflow-wrap:anywhere] text-muted-foreground">
                      {entry.appId}
                    </div>
                  </div>
                  <div className="flex items-center gap-1">
                    <NativeSelect
                      wrapperClassName="min-w-44"
                      aria-label={t("style.appFormats.formatFor", { app })}
                      value={entry.format}
                      onChange={(event) =>
                        change(entry.appId, event.target.value as TranscriptFormat)
                      }
                    >
                      {formatOptions}
                    </NativeSelect>
                    <Button
                      variant="ghost"
                      size="sm"
                      aria-label={t("style.appFormats.removeApp", { app })}
                      onClick={() => remove(entry.appId)}
                    >
                      {t("style.appFormats.remove")}
                    </Button>
                  </div>
                </li>
              );
            })}
          </ul>
        )}
        {choice ? (
          <Row className="justify-start gap-x-4">
            <label className="flex items-center gap-2">
              <span className="text-sm font-medium">{t("style.appFormats.app")}</span>
              <NativeSelect
                wrapperClassName="min-w-36"
                value={choice.id}
                onChange={(event) => setPicked(event.target.value)}
              >
                {available.map((app) => (
                  <option key={app.id} value={app.id}>
                    {app.name}
                  </option>
                ))}
              </NativeSelect>
            </label>
            <label className="flex items-center gap-2">
              <span className="text-sm font-medium">{t("style.appFormats.format")}</span>
              <NativeSelect
                wrapperClassName="min-w-44"
                value={format}
                onChange={(event) => setFormat(event.target.value as TranscriptFormat)}
              >
                {formatOptions}
              </NativeSelect>
            </label>
            <Button className="ml-auto" onClick={add}>
              {t("style.appFormats.add")}
            </Button>
          </Row>
        ) : (
          known.length === 0 && (
            <p className="px-4 py-3 text-muted-foreground">{t("style.appFormats.noApps")}</p>
          )
        )}
      </Group>
      <Note>{t("style.appFormats.footnote")}</Note>
    </Section>
  );
}
