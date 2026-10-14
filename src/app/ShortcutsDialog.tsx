import { Info, Plus, Trash2, X } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { keycap, recordingKeycap } from "@/components/ui/kbd";
import { cn } from "@/lib/utils";
import {
  errorMessage,
  EVENTS,
  getShortcuts,
  removeShortcut,
  resetShortcuts,
  sameAction,
  type ShortcutList,
  type ShortcutRow,
} from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";
import { Dialog, DialogFooter } from "./Dialog";
import { useShortcutRecording, type ShortcutRecording } from "./useShortcutRecording";

interface ShortcutsDialogProps {
  onClose: () => void;
}

/**
 * Hands-free also starts by double-tapping push to talk (CORE-05), so its row
 * says so with the user's own keys (KEYS-11): "Or double-tap fn".
 */
function useDoubleTapHint() {
  const { t, i18n } = useTranslation();
  return (handsFree: ShortcutRow, rows: ShortcutRow[]): string | null => {
    const keys = rows.find((row) => row.action === "pushToTalk")?.shortcuts ?? [];
    if (keys.length === 0) {
      return null;
    }
    const list = new Intl.ListFormat(i18n.language, { type: "disjunction" }).format(keys);
    return handsFree.shortcuts.length > 0
      ? t("shortcuts.doubleTap", { keys: list })
      : t("shortcuts.doubleTapOnly", { keys: list });
  };
}

/** Every action and its bindings (KEYS-01): click one to record a replacement, + to add one. */
export function ShortcutsDialog({ onClose }: ShortcutsDialogProps) {
  const { t } = useTranslation();
  const doubleTapHint = useDoubleTapHint();
  const [list, setList] = useState<ShortcutList | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [confirmReset, setConfirmReset] = useState(false);

  const load = useCallback(() => {
    getShortcuts()
      .then(setList)
      .catch((err: unknown) => setProblem(errorMessage(err)));
  }, []);
  useEffect(load, [load]);
  useTauriEvent(EVENTS.settings, load);
  const recording = useShortcutRecording(() => true, load);

  const guard = (work: () => void) => () => {
    if (recording.active) {
      setProblem(t("shortcuts.busy"));
      return;
    }
    setProblem(null);
    work();
  };

  const run = (work: Promise<void>) =>
    work.then(load).catch((err: unknown) => setProblem(errorMessage(err)));

  const close = () => {
    if (recording.active) {
      recording.cancel();
    }
    onClose();
  };

  const { result } = recording;
  const warning = result?.kind === "saved" ? result.warning : list?.warning;
  return (
    <Dialog title={t("shortcuts.title")} onClose={close} wide>
      {list?.secureInput && (
        <p
          className="flex items-start gap-2 rounded-lg border bg-muted/50 px-3.5 py-2.5 text-sm"
          role="status"
        >
          <Info aria-hidden="true" className="mt-px size-3.5 shrink-0 text-muted-foreground" />
          {t("shortcuts.secureInput", { app: list.secureInput })}
        </p>
      )}
      <ul className="flex flex-col divide-y divide-separator rounded-xl border bg-card shadow-xs empty:hidden">
        {list?.rows.map((row) => (
          <ShortcutItem
            key={
              typeof row.action === "string" ? row.action : `custom-${row.action.customTransform}`
            }
            row={row}
            hint={row.action === "handsFree" ? doubleTapHint(row, list.rows) : null}
            recording={recording}
            guard={guard}
            onRemove={(index) => void run(removeShortcut(row.action, index))}
          />
        ))}
      </ul>
      <div className="flex flex-col gap-1 empty:hidden" aria-live="polite">
        {result?.kind === "saved" && <p className="text-sm text-success">{result.text}</p>}
        {result?.kind === "failed" && (
          <p className="text-sm text-destructive" role="alert">
            {result.text}
          </p>
        )}
        {warning && <p className="text-sm text-warning">{warning}</p>}
        {problem && (
          <p className="text-sm text-destructive" role="alert">
            {problem}
          </p>
        )}
      </div>
      {confirmReset ? (
        <DialogFooter className="items-center">
          <p className="mr-auto min-w-0 flex-1 basis-56 text-sm">{t("shortcuts.resetConfirm")}</p>
          <Button onClick={() => setConfirmReset(false)}>{t("common.cancel")}</Button>
          <Button
            variant="destructive"
            onClick={() => {
              setConfirmReset(false);
              void run(resetShortcuts());
            }}
          >
            {t("shortcuts.reset")}
          </Button>
        </DialogFooter>
      ) : (
        <DialogFooter>
          <Button className="mr-auto" onClick={guard(() => setConfirmReset(true))}>
            {t("shortcuts.resetToDefault")}
          </Button>
          <Button variant="default" onClick={close}>
            {t("dialog.done")}
          </Button>
        </DialogFooter>
      )}
    </Dialog>
  );
}

interface ShortcutItemProps {
  row: ShortcutRow;
  /** Another way to start the action, shown under its name. */
  hint: string | null;
  recording: ShortcutRecording;
  guard: (work: () => void) => () => void;
  onRemove: (index: number) => void;
}

function ShortcutItem({ row, hint, recording, guard, onRemove }: ShortcutItemProps) {
  const { t } = useTranslation();
  const { active } = recording;
  const mine = active !== null && sameAction(active.action, row.action);
  const field = mine ? (
    <RecordingField preview={active.preview} onCancel={recording.cancel} />
  ) : null;
  const canRemove = !(row.required && row.shortcuts.length === 1);
  return (
    <li className="group/item flex min-h-(--row-min-h) flex-wrap items-center justify-between gap-x-4 gap-y-2 py-2 pr-2 pl-4">
      <span className="flex min-w-0 flex-1 basis-40 flex-col gap-0.5">
        <span>{row.name}</span>
        {hint && <span className="text-sm text-muted-foreground">{hint}</span>}
      </span>
      <div className="flex flex-wrap items-center justify-end gap-x-1 gap-y-1.5">
        {row.shortcuts.map((label, index) =>
          mine && active.index === index ? (
            <span key={index}>{field}</span>
          ) : (
            <span key={index} className="inline-flex items-center">
              <button
                type="button"
                className={keycap}
                aria-label={t("shortcuts.change", { action: row.name, shortcut: label })}
                onClick={guard(() => recording.start(row.action, index))}
              >
                {label}
              </button>
              {canRemove && (
                <Button
                  variant="ghost"
                  size="icon-sm"
                  className="text-muted-foreground opacity-0 transition-[opacity,background-color,color] group-focus-within/item:opacity-100 group-hover/item:opacity-100 hover:text-destructive"
                  aria-label={t("shortcuts.remove", { shortcut: label, action: row.name })}
                  title={t("shortcuts.removeTitle")}
                  onClick={guard(() => onRemove(index))}
                >
                  <Trash2 aria-hidden="true" />
                </Button>
              )}
            </span>
          ),
        )}
        {mine && active.index === null && field}
        {!(mine && active.index === null) &&
          (row.shortcuts.length === 0 ? (
            <button
              type="button"
              className="inline-flex h-(--control-h-sm) items-center rounded-md border border-dashed border-input px-2.5 text-sm text-muted-foreground transition-colors duration-150 outline-none hover:bg-accent hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring/70"
              onClick={guard(() => recording.start(row.action, null))}
            >
              {t("shortcuts.add")}
            </button>
          ) : (
            row.shortcuts.length < row.max && (
              <Button
                variant="ghost"
                size="icon-sm"
                className="text-muted-foreground hover:text-foreground"
                aria-label={t("shortcuts.addAnother", { action: row.name })}
                title={t("shortcuts.addAnotherTitle")}
                onClick={guard(() => recording.start(row.action, null))}
              >
                <Plus aria-hidden="true" />
              </Button>
            )
          ))}
      </div>
    </li>
  );
}

function RecordingField({ preview, onCancel }: { preview: string | null; onCancel: () => void }) {
  const { t } = useTranslation();
  return (
    <span className={cn(keycap, recordingKeycap, "gap-1.5 pr-0.5 pl-2")} role="status">
      <span aria-hidden="true" className="size-1.5 shrink-0 animate-pulse rounded-full bg-brand" />
      <span>{preview ?? t("shortcuts.pressKey")}</span>
      <Button
        variant="ghost"
        size="icon-sm"
        className="size-5 rounded-[5px] text-brand-strong hover:bg-brand/12 [&_svg:not([class*='size-'])]:size-3"
        aria-label={t("shortcuts.cancelRecording")}
        onClick={onCancel}
      >
        <X aria-hidden="true" />
      </Button>
    </span>
  );
}
