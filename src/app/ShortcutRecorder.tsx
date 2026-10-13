import { useRef, useState, type KeyboardEvent } from "react";
import { errorMessage } from "../lib/ipc";
import { isModifierCode, keyName, shortcutLabel, shortcutText } from "../lib/shortcuts";

interface ShortcutRecorderProps {
  label: string;
  value: string | null;
  /** Saves a shortcut, or clears it with null; rejects with the message to show. */
  onSave: (shortcut: string | null) => Promise<void>;
}

/** Click, then press the keys: the shortcut saves when they are released. Esc alone cancels. */
export function ShortcutRecorder({ label, value, onSave }: ShortcutRecorderProps) {
  const [recording, setRecording] = useState(false);
  const [preview, setPreview] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const candidate = useRef<string | null>(null);

  const stop = () => {
    setRecording(false);
    setPreview(null);
    candidate.current = null;
  };

  const save = (shortcut: string | null) => {
    stop();
    onSave(shortcut)
      .then(() => setError(null))
      .catch((err: unknown) => setError(errorMessage(err)));
  };

  const onKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (!recording) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    const plain = !event.ctrlKey && !event.altKey && !event.shiftKey && !event.metaKey;
    if (event.code === "Escape" && plain) {
      stop();
      return;
    }
    const modifier = isModifierCode(event.code);
    const key = modifier ? null : keyName(event.code);
    if (!modifier && key === null) {
      setError("That key can't be used in a shortcut.");
      return;
    }
    candidate.current = shortcutText(event, key);
    setPreview(candidate.current);
  };

  const onKeyUp = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (!recording || candidate.current === null) {
      return;
    }
    event.preventDefault();
    const released = !event.ctrlKey && !event.altKey && !event.shiftKey && !event.metaKey;
    if (!isModifierCode(event.code) || released) {
      save(candidate.current);
    }
  };

  const shown = recording
    ? preview
      ? shortcutLabel(preview)
      : "Press keys…"
    : value
      ? shortcutLabel(value)
      : "Not set";

  return (
    <div className="shortcut-row">
      <div className="row">
        <span>{label}</span>
        <div className="row-end">
          <button
            type="button"
            className={recording ? "recorder recording" : "recorder"}
            aria-label={`${label} shortcut: ${shown}`}
            aria-pressed={recording}
            onClick={() => {
              setError(null);
              setRecording(true);
            }}
            onKeyDown={onKeyDown}
            onKeyUp={onKeyUp}
            onBlur={stop}
          >
            {shown}
          </button>
          {value && !recording && (
            <button
              type="button"
              aria-label={`Remove ${label} shortcut`}
              onClick={() => save(null)}
            >
              Remove
            </button>
          )}
        </div>
      </div>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
