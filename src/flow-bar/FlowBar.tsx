import { useEffect, useRef, useState } from "react";
import {
  EVENTS,
  flowBarReady,
  getDictationState,
  type Answer,
  type DictationState,
  type Notice,
} from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";
import { AnswerCard } from "./AnswerCard";
import { NoticeCard } from "./NoticeCard";
import { Pill } from "./Pill";
import { useContentSize } from "./useContentSize";

export const WAVE_BARS = 18;

const NOTICE_DURATION_MS: Record<Notice["level"], number> = {
  info: 4000,
  warning: 6000,
  error: 8000,
};

/** An answer stays until dismissed, a new session starts, or this long. */
const ANSWER_DURATION_MS = 90_000;

const silence = (): number[] => Array.from({ length: WAVE_BARS }, () => 0);

export function FlowBar() {
  const [state, setState] = useState<DictationState>({ phase: "idle" });
  const [levels, setLevels] = useState<number[]>(silence);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [answer, setAnswer] = useState<Answer | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);
  useContentSize(rootRef);

  useEffect(() => {
    let cancelled = false;
    getDictationState()
      .then((current) => !cancelled && setState(current))
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);

  useTauriEvent<DictationState>(EVENTS.state, (next) => {
    setState((previous) => {
      if (next.phase === "recording" && previous.phase !== "recording") {
        setLevels(silence());
        setAnswer(null);
      }
      return next;
    });
  });
  useTauriEvent<number>(EVENTS.level, (level) => {
    setLevels((previous) => [...previous.slice(1), level]);
  });
  useTauriEvent<Notice>(EVENTS.notice, setNotice, () => void flowBarReady());
  useTauriEvent<Answer>(EVENTS.answer, setAnswer);

  useEffect(() => {
    if (!answer) {
      return;
    }
    const timer = setTimeout(
      () => setAnswer((current) => (current?.id === answer.id ? null : current)),
      ANSWER_DURATION_MS,
    );
    return () => clearTimeout(timer);
  }, [answer]);

  useEffect(() => {
    if (!notice) {
      return;
    }
    const timer = setTimeout(
      () => setNotice((current) => (current?.id === notice.id ? null : current)),
      NOTICE_DURATION_MS[notice.level],
    );
    return () => clearTimeout(timer);
  }, [notice]);

  return (
    <div className="flow-bar" ref={rootRef}>
      {answer && <AnswerCard answer={answer} onDismiss={() => setAnswer(null)} />}
      {notice && <NoticeCard notice={notice} onDismiss={() => setNotice(null)} />}
      <Pill state={state} levels={levels} error={notice?.level === "error"} />
    </div>
  );
}
