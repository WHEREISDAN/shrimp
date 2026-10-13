import { useCallback, useEffect, useState } from "react";
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

const BUSY = "Finish or cancel the recording first.";

interface ShortcutsDialogProps {
  onClose: () => void;
}

/** Every action and its bindings (KEYS-01): click one to record a replacement, + to add one. */
export function ShortcutsDialog({ onClose }: ShortcutsDialogProps) {
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
      setProblem(BUSY);
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
    <Dialog title="Shortcuts" onClose={close} wide>
      {list?.secureInput && (
        <p className="banner" role="status">
          Secure Keyboard Entry is on (macOS names {list.secureInput}), so shortcuts with letters or
          other keys don't work until it's off. fn and other modifier-only shortcuts still work.
        </p>
      )}
      <ul className="shortcut-list">
        {list?.rows.map((row) => (
          <ShortcutItem
            key={
              typeof row.action === "string" ? row.action : `custom-${row.action.customTransform}`
            }
            row={row}
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
          <p className="grow">Reset every shortcut to its default? This can't be undone.</p>
          <button type="button" onClick={() => setConfirmReset(false)}>
            Cancel
          </button>
          <button
            type="button"
            className="danger"
            onClick={() => {
              setConfirmReset(false);
              void run(resetShortcuts());
            }}
          >
            Reset
          </button>
        </div>
      ) : (
        <div className="dialog-actions">
          <button type="button" onClick={guard(() => setConfirmReset(true))}>
            Reset to default
          </button>
          <button type="button" className="primary" onClick={close}>
            Done
          </button>
        </div>
      )}
    </Dialog>
  );
}

interface ShortcutItemProps {
  row: ShortcutRow;
  recording: ShortcutRecording;
  guard: (work: () => void) => () => void;
  onRemove: (index: number) => void;
}

function ShortcutItem({ row, recording, guard, onRemove }: ShortcutItemProps) {
  const { active } = recording;
  const mine = active !== null && sameAction(active.action, row.action);
  const field = mine ? (
    <RecordingField preview={active.preview} onCancel={recording.cancel} />
  ) : null;
  const canRemove = !(row.required && row.shortcuts.length === 1);
  return (
    <li className="shortcut-item">
      <span className="shortcut-name">{row.name}</span>
      <div className="shortcut-bindings">
        {row.shortcuts.map((label, index) =>
          mine && active.index === index ? (
            <span key={index}>{field}</span>
          ) : (
            <span key={index} className="binding">
              <button
                type="button"
                className="binding-key"
                aria-label={`${row.name}: ${label}. Change`}
                onClick={guard(() => recording.start(row.action, index))}
              >
                {label}
              </button>
              {canRemove && (
                <button
                  type="button"
                  className="binding-remove"
                  aria-label={`Remove shortcut ${label} from ${row.name}`}
                  title="Remove shortcut"
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
              Click to add a shortcut
            </button>
          ) : (
            row.shortcuts.length < row.max && (
              <button
                type="button"
                className="binding-add"
                aria-label={`Add another shortcut for ${row.name}`}
                title="Add another"
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
  return (
    <span className="binding recording" role="status">
      <span>{preview ?? "Press key or mouse button"}</span>
      <button
        type="button"
        className="binding-remove"
        aria-label="Cancel recording"
        onClick={onCancel}
      >
        ✕
      </button>
    </span>
  );
}
