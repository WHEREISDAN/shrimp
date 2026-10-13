import { useCallback, useEffect, useState } from "react";
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
import { useShortcutRecording } from "./useShortcutRecording";

interface ShortcutRecorderProps {
  action: ShortcutAction;
  label: string;
}

/** An action's first shortcut: click, then press the keys or a mouse button. More are in the Shortcuts dialog. */
export function ShortcutRecorder({ action, label }: ShortcutRecorderProps) {
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
  const shown = recording.active
    ? (recording.active.preview ?? "Press key or mouse button")
    : (value ?? "Not set");
  const problem = error ?? (recording.result?.kind === "failed" ? recording.result.text : null);

  return (
    <div className="shortcut-row">
      <div className="row">
        <span>{label}</span>
        <div className="row-end">
          <button
            type="button"
            className={recording.active ? "recorder recording" : "recorder"}
            aria-label={`${label} shortcut: ${shown}`}
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
            {shown}
          </button>
          {value && !recording.active && (
            <button
              type="button"
              aria-label={`Remove ${label} shortcut`}
              onClick={() =>
                void removeShortcut(action, 0)
                  .then(load)
                  .catch((err: unknown) => setError(errorMessage(err)))
              }
            >
              Remove
            </button>
          )}
        </div>
      </div>
      {problem && (
        <p className="error" role="alert">
          {problem}
        </p>
      )}
    </div>
  );
}
