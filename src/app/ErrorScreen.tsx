import { DatabaseZap, FolderInput, HardDrive, TriangleAlert, type LucideIcon } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { DragBand } from "@/components/drag-band";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
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

const ICONS: Record<ReturnType<typeof problemKind>, { icon: LucideIcon; tone: string }> = {
  misplaced: { icon: FolderInput, tone: "bg-brand/12 text-brand-strong" },
  data: { icon: DatabaseZap, tone: "bg-destructive/10 text-destructive" },
  storage: { icon: HardDrive, tone: "bg-warning/12 text-warning" },
  crash: { icon: TriangleAlert, tone: "bg-warning/12 text-warning" },
};

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

  const { icon: Icon, tone } = ICONS[kind];
  return (
    <main
      className="error-screen flex min-h-full flex-col items-center justify-center overflow-y-auto bg-background px-8 py-16"
      aria-labelledby="error-title"
    >
      <DragBand />
      <div className="error-card flex w-full max-w-[26rem] animate-pop-in flex-col items-center gap-3 text-center">
        <span
          aria-hidden="true"
          className={cn("mb-2 grid size-14 place-items-center rounded-2xl", tone)}
        >
          <Icon className="size-7" strokeWidth={1.75} />
        </span>
        <h1
          id="error-title"
          ref={headingRef}
          tabIndex={-1}
          className="font-display text-2xl font-bold tracking-[-0.01em] outline-none"
        >
          {title}
        </h1>
        <p className="text-muted-foreground">{body}</p>
        {extra && <p className="text-sm text-muted-foreground">{extra}</p>}
        {health.crash?.details && kind === "crash" && (
          <details className="mt-1 w-full rounded-lg border bg-card text-left">
            <summary className="px-3 py-2 text-sm text-muted-foreground">
              {t("errors.crashDetails")}
            </summary>
            <pre className="m-0 max-h-40 overflow-auto border-t px-3 py-2 font-mono text-xs whitespace-pre-wrap select-text">
              {health.crash.details}
            </pre>
          </details>
        )}
        {error && (
          <p className="text-sm text-destructive" role="alert">
            {error}
          </p>
        )}
        <div className="mt-3 flex flex-wrap justify-center gap-2">
          {actions.map((action) => (
            <Button
              key={action.label}
              size="lg"
              variant={action.primary ? "default" : "outline"}
              disabled={busy}
              onClick={() => run(action.run)}
            >
              {action.label}
            </Button>
          ))}
        </div>
      </div>
    </main>
  );
}
