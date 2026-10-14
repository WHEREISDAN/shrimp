import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  checkStorage,
  dismissCrash,
  errorMessage,
  moveToApplications,
  openStorageSettings,
  quitApp,
  repairData,
  type AppHealth,
} from "../lib/ipc";

interface ErrorScreenProps {
  health: AppHealth;
}

/** Whether the main window shows an error screen instead of the app (APP-09, APP-11). */
export const hasProblem = (health: AppHealth | null): health is AppHealth =>
  Boolean(health && (health.misplaced || health.data || health.storage || health.crash));

/** Which screen shows: the app outside Applications, a damaged database, a full disk, a crash. */
export const problemKind = (health: AppHealth) =>
  health.misplaced ? "misplaced" : health.data ? "data" : health.storage ? "storage" : "crash";

/** The app outside Applications, a damaged database, a full disk or a crash, each with its way out. */
export function ErrorScreen({ health }: ErrorScreenProps) {
  const { t, i18n } = useTranslation();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const headingRef = useRef<HTMLHeadingElement>(null);
  const kind = problemKind(health);

  useEffect(() => headingRef.current?.focus(), []);

  const run = (action: () => Promise<unknown>) => {
    setBusy(true);
    setError(null);
    action()
      .catch((err: unknown) => setError(errorMessage(err)))
      .finally(() => setBusy(false));
  };

  let title: string;
  let body: string;
  let extra: string | null = null;
  let actions: { label: string; primary?: boolean; run: () => Promise<unknown> }[];
  if (health.misplaced) {
    title = t("errors.moveTitle");
    body = t("errors.moveBody");
    actions = [
      { label: t("errors.quit"), run: quitApp },
      { label: t("errors.moveToApplications"), primary: true, run: moveToApplications },
    ];
  } else if (health.data) {
    title = t("errors.dataTitle");
    body = t("errors.dataBody");
    const backup = health.data.backup;
    extra = backup
      ? t("errors.dataBackup", {
          date: new Intl.DateTimeFormat(i18n.language, { dateStyle: "long" }).format(
            new Date(`${backup}T12:00:00`),
          ),
        })
      : null;
    actions = [
      ...(backup
        ? [
            {
              label: t("errors.restoreBackup"),
              primary: true,
              run: () => repairData("restoreBackup"),
            },
          ]
        : []),
      { label: t("errors.startFresh"), primary: !backup, run: () => repairData("startFresh") },
    ];
  } else if (health.storage) {
    title = t("errors.storageTitle");
    body = t("errors.storageBody");
    extra = t("errors.storageFree", {
      free: new Intl.NumberFormat(i18n.language, {
        style: "unit",
        unit: "megabyte",
        maximumFractionDigits: 0,
      }).format(health.storage.freeBytes / 1_000_000),
    });
    actions = [
      { label: t("errors.openStorage"), run: openStorageSettings },
      { label: t("errors.checkAgain"), primary: true, run: checkStorage },
    ];
  } else {
    title = t("errors.crashTitle");
    body = t("errors.crashBody");
    actions = [{ label: t("errors.continue"), primary: true, run: dismissCrash }];
  }

  return (
    <main className="error-screen" aria-labelledby="error-title">
      <div className="error-card">
        <h1 id="error-title" ref={headingRef} tabIndex={-1}>
          {title}
        </h1>
        <p>{body}</p>
        {extra && <p className="muted">{extra}</p>}
        {health.crash?.details && kind === "crash" && (
          <details>
            <summary>{t("errors.crashDetails")}</summary>
            <pre>{health.crash.details}</pre>
          </details>
        )}
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        <div className="dialog-actions">
          {actions.map((action) => (
            <button
              key={action.label}
              type="button"
              className={action.primary ? "primary" : undefined}
              disabled={busy}
              onClick={() => run(action.run)}
            >
              {action.label}
            </button>
          ))}
        </div>
      </div>
    </main>
  );
}
