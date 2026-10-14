import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import {
  errorMessage,
  EVENTS,
  getShortcuts,
  removeShortcut,
  sameAction,
  type ShortcutAction,
  type ShortcutRow,
} from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";
import { keycap, recordingKeycap } from "@/components/ui/kbd";
import { useShortcutRecording } from "./useShortcutRecording";

interface ShortcutRecorderProps {
  action: ShortcutAction;
  label: string;
}

/** An action's first shortcut: click, then press the keys or a mouse button. More are in the Shortcuts dialog. */
export function ShortcutRecorder({ action, label }: ShortcutRecorderProps) {
  const { t } = useTranslation();
  const [row, setRow] = useState<ShortcutRow | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    getShortcuts()
      .then((list) =>
        setRow(list.rows.find((candidate) => sameAction(candidate.action, action)) ?? null),
      )
      .catch(() => undefined);
  }, [action]);
  useEffect(load, [load]);
  useTauriEvent(EVENTS.settings, load);
  const recording = useShortcutRecording((other) => sameAction(other, action), load);

  const value = row?.shortcuts[0] ?? null;
  const shown: string = recording.active
    ? (recording.active.preview ?? t("shortcuts.pressKey"))
    : (value ?? t("shortcuts.notSet"));
  const problem = error ?? (recording.result?.kind === "failed" ? recording.result.text : null);

  return (
    <div className="flex w-full min-w-0 flex-col gap-1.5">
      <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
        <span className="min-w-0 flex-1 basis-40">{label}</span>
        <div className="flex flex-wrap items-center gap-2">
          <button
            type="button"
            className={cn(
              keycap,
              "min-w-[6.5rem]",
              recording.active ? recordingKeycap : !value && "font-normal text-muted-foreground",
            )}
            aria-label={t("shortcuts.recorder", { label, shortcut: shown })}
            aria-pressed={recording.active !== null}
            onClick={() => {
              setError(null);
              if (recording.active) {
                recording.cancel();
              } else {
                recording.start(action, value ? 0 : null);
              }
            }}
          >
            {recording.active && (
              <span
                aria-hidden="true"
                className="size-1.5 shrink-0 animate-pulse rounded-full bg-brand"
              />
            )}
            {shown}
          </button>
          {value && !recording.active && (
            <Button
              variant="ghost"
              size="sm"
              className="text-muted-foreground hover:text-foreground"
              aria-label={t("shortcuts.removeOneLabel", { label })}
              onClick={() =>
                void removeShortcut(action, 0)
                  .then(load)
                  .catch((err: unknown) => setError(errorMessage(err)))
              }
            >
              {t("shortcuts.removeOne")}
            </Button>
          )}
        </div>
      </div>
      {problem && (
        <p className="text-sm text-destructive" role="alert">
          {problem}
        </p>
      )}
    </div>
  );
}
