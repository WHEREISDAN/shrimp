import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
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
import { Dialog } from "./Dialog";
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
  return (
    <Dialog title={t("shortcuts.title")} onClose={close} wide>
      {list?.secureInput && (
        <p className="banner" role="status">
          {t("shortcuts.secureInput", { app: list.secureInput })}
        </p>
      )}
      <ul className="shortcut-list">
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
      <div className="shortcut-status" aria-live="polite">
        {result?.kind === "saved" && <p className="ok">{result.text}</p>}
        {result?.kind === "failed" && (
          <p className="error" role="alert">
            {result.text}
          </p>
        )}
        {(result?.kind === "saved" ? result.warning : list?.warning) && (
          <p className="warning">{result?.kind === "saved" ? result.warning : list?.warning}</p>
        )}
        {problem && (
          <p className="error" role="alert">
            {problem}
          </p>
        )}
      </div>
      {confirmReset ? (
        <div className="dialog-actions">
          <p className="grow">{t("shortcuts.resetConfirm")}</p>
          <button type="button" onClick={() => setConfirmReset(false)}>
            {t("common.cancel")}
          </button>
          <button
            type="button"
            className="danger"
            onClick={() => {
              setConfirmReset(false);
              void run(resetShortcuts());
            }}
          >
            {t("shortcuts.reset")}
          </button>
        </div>
      ) : (
        <div className="dialog-actions">
          <button type="button" onClick={guard(() => setConfirmReset(true))}>
            {t("shortcuts.resetToDefault")}
          </button>
          <button type="button" className="primary" onClick={close}>
            {t("dialog.done")}
          </button>
        </div>
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
    <li className="shortcut-item">
      <span className="shortcut-label">
        <span className="shortcut-name">{row.name}</span>
        {hint && <span className="shortcut-hint">{hint}</span>}
      </span>
      <div className="shortcut-bindings">
        {row.shortcuts.map((label, index) =>
          mine && active.index === index ? (
            <span key={index}>{field}</span>
          ) : (
            <span key={index} className="binding">
              <button
                type="button"
                className="binding-key"
                aria-label={t("shortcuts.change", { action: row.name, shortcut: label })}
                onClick={guard(() => recording.start(row.action, index))}
              >
                {label}
              </button>
              {canRemove && (
                <button
                  type="button"
                  className="binding-remove"
                  aria-label={t("shortcuts.remove", { shortcut: label, action: row.name })}
                  title={t("shortcuts.removeTitle")}
                  onClick={guard(() => onRemove(index))}
                >
                  <svg viewBox="0 0 16 16" aria-hidden="true">
                    <path d="M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.7 8.5h5.6l.7-8.5" />
                  </svg>
                </button>
              )}
            </span>
          ),
        )}
        {mine && active.index === null && field}
        {!(mine && active.index === null) &&
          (row.shortcuts.length === 0 ? (
            <button
              type="button"
              className="binding-empty"
              onClick={guard(() => recording.start(row.action, null))}
            >
              {t("shortcuts.add")}
            </button>
          ) : (
            row.shortcuts.length < row.max && (
              <button
                type="button"
                className="binding-add"
                aria-label={t("shortcuts.addAnother", { action: row.name })}
                title={t("shortcuts.addAnotherTitle")}
                onClick={guard(() => recording.start(row.action, null))}
              >
                +
              </button>
            )
          ))}
      </div>
    </li>
  );
}

function RecordingField({ preview, onCancel }: { preview: string | null; onCancel: () => void }) {
  const { t } = useTranslation();
  return (
    <span className="binding recording" role="status">
      <span>{preview ?? t("shortcuts.pressKey")}</span>
      <button
        type="button"
        className="binding-remove"
        aria-label={t("shortcuts.cancelRecording")}
        onClick={onCancel}
      >
        ✕
      </button>
    </span>
  );
}
