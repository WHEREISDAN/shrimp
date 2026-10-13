import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  cancelShortcutRecording,
  errorMessage,
  EVENTS,
  recordShortcut,
  sameAction,
  type RecordingStatus,
  type ShortcutAction,
} from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";

/** The binding being recorded: its action, its position (null adds one) and what is held so far. */
export interface ActiveRecording {
  action: ShortcutAction;
  index: number | null;
  preview: string | null;
}

/** The outcome of the last recording, shown next to the shortcut. */
export type RecordingResult =
  { kind: "saved"; text: string; warning: string | null } | { kind: "failed"; text: string };

export interface ShortcutRecording {
  active: ActiveRecording | null;
  result: RecordingResult | null;
  start: (action: ShortcutAction, index: number | null) => void;
  cancel: () => void;
}

/** The hooks see a key before the page does, so keys stay swallowed briefly after recording ends. */
const GRACE_MS = 500;

/**
 * Records shortcuts through Shrimp's keyboard and mouse hooks, which see Fn, Caps Lock and
 * mouse buttons. Keys save on release, mouse buttons on press; Esc alone or a click cancels.
 * `owns` limits the hook to the actions this view shows.
 */
export function useShortcutRecording(
  owns: (action: ShortcutAction) => boolean,
  onChanged: () => void,
): ShortcutRecording {
  const { t } = useTranslation();
  const [active, setActive] = useState<ActiveRecording | null>(null);
  const [result, setResult] = useState<RecordingResult | null>(null);
  const recordingRef = useRef(false);
  const endedAt = useRef(0);
  useEffect(() => {
    if (recordingRef.current && !active) {
      endedAt.current = Date.now();
    }
    recordingRef.current = active !== null;
  }, [active]);

  useTauriEvent<RecordingStatus>(EVENTS.recording, (status) => {
    if (!owns(status.action)) {
      return;
    }
    switch (status.state) {
      case "recording":
        setActive((current) =>
          current && sameAction(current.action, status.action)
            ? { ...current, preview: status.preview }
            : current,
        );
        return;
      case "saved":
        setActive(null);
        setResult({ kind: "saved", text: t("shortcuts.saved"), warning: status.warning });
        onChanged();
        return;
      case "failed":
        setActive(null);
        setResult({ kind: "failed", text: status.message });
        return;
      case "cancelled":
        setActive(null);
        return;
    }
  });

  const cancel = useCallback(() => {
    void cancelShortcutRecording().catch(() => undefined);
    setActive(null);
  }, []);

  const start = useCallback((action: ShortcutAction, index: number | null) => {
    setResult(null);
    setActive({ action, index, preview: null });
    recordShortcut(action, index).catch((err: unknown) => {
      setActive(null);
      setResult({ kind: "failed", text: errorMessage(err) });
    });
  }, []);

  // The hooks read the keys; here they must not press buttons or close the dialog.
  useEffect(() => {
    const swallow = (event: KeyboardEvent) => {
      if (recordingRef.current || Date.now() - endedAt.current < GRACE_MS) {
        event.preventDefault();
        event.stopPropagation();
      }
    };
    window.addEventListener("keydown", swallow, true);
    window.addEventListener("keyup", swallow, true);
    return () => {
      window.removeEventListener("keydown", swallow, true);
      window.removeEventListener("keyup", swallow, true);
    };
  }, []);

  useEffect(() => {
    if (!active) {
      return;
    }
    window.addEventListener("blur", cancel);
    return () => window.removeEventListener("blur", cancel);
  }, [active, cancel]);

  return { active, result, start, cancel };
}
