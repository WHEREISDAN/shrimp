import { useEffect, useReducer, useRef, useState } from "react";
import {
  EVENTS,
  flowBarReady,
  getDictationState,
  getSettings,
  type Settings,
  type Answer,
  type DictationState,
  type Notice,
  type TransformView,
} from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";
import { AnswerCard } from "./AnswerCard";
import { ChangesCard } from "./ChangesCard";
import { FlowBarMenu } from "./FlowBarMenu";
import { LanguageSwitch } from "./LanguageSwitch";
import { NoticeCard } from "./NoticeCard";
import { EMPTY_QUEUE, noticeQueue } from "./noticeQueue";
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
  const [notices, dispatch] = useReducer(noticeQueue, EMPTY_QUEUE);
  const notice = notices.current;
  const [answer, setAnswer] = useState<Answer | null>(null);
  const [changes, setChanges] = useState<TransformView | null>(null);
  const [hovered, setHovered] = useState(false);
  const [menu, setMenu] = useState(false);
  const [placement, setPlacement] = useState<Settings["flowBar"] | null>(null);
  const [privacy, setPrivacy] = useState(false);
  const [now, setNow] = useState(() => Date.now() / 1000);
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

  useEffect(() => {
    let cancelled = false;
    getSettings()
      .then((settings) => {
        if (!cancelled) {
          setPlacement(settings.flowBar);
          setPrivacy(settings.privacy.privacyMode);
        }
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);
  useTauriEvent<Settings>(EVENTS.settings, (settings) => {
    setPlacement(settings.flowBar);
    setPrivacy(settings.privacy.privacyMode);
    setNow(Date.now() / 1000);
  });

  // Hidden for an hour: show the resting bar again when the hour is up (BAR-04).
  const hiddenUntil = placement?.hiddenUntil ?? 0;
  useEffect(() => {
    const wait = hiddenUntil - Date.now() / 1000;
    if (wait <= 0) {
      return;
    }
    const timer = setTimeout(() => setNow(Date.now() / 1000), wait * 1000 + 50);
    return () => clearTimeout(timer);
  }, [hiddenUntil]);

  useTauriEvent<DictationState>(EVENTS.state, (next) => {
    setState((previous) => {
      if (next.phase === "recording" && previous.phase !== "recording") {
        setLevels(silence());
        setAnswer(null);
        setChanges(null);
      }
      return next;
    });
  });
  useTauriEvent<number>(EVENTS.level, (level) => {
    setLevels((previous) => [...previous.slice(1), level]);
  });
  useTauriEvent<Notice>(
    EVENTS.notice,
    (next) => dispatch({ type: "add", notice: next }),
    () => void flowBarReady(),
  );
  useTauriEvent<number>(EVENTS.dismiss, (id) => dispatch({ type: "remove", id }));
  useTauriEvent<Answer>(EVENTS.answer, setAnswer);
  useTauriEvent<TransformView>(EVENTS.changes, setChanges);
  // The panel never becomes key, so WebKit sends no hover events; Rust tracks the cursor.
  const hover = (inside: boolean) => {
    setHovered(inside);
    if (!inside) {
      setMenu(false);
    }
  };
  useTauriEvent<boolean>(EVENTS.flowBarHover, hover);

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
      () => dispatch({ type: "remove", id: notice.id }),
      NOTICE_DURATION_MS[notice.level],
    );
    return () => clearTimeout(timer);
  }, [notice]);

  const resting = (placement?.alwaysShow ?? true) && now >= hiddenUntil;
  const edge = placement?.edge ?? "bottom";
  return (
    <div
      className={`flow-bar edge-${edge}`}
      ref={rootRef}
      onMouseEnter={() => hover(true)}
      onMouseLeave={() => hover(false)}
      onContextMenu={(event) => {
        event.preventDefault();
        setMenu(true);
      }}
    >
      {menu && <FlowBarMenu onClose={() => setMenu(false)} />}
      {changes && <ChangesCard view={changes} onClose={() => setChanges(null)} />}
      {answer && <AnswerCard answer={answer} onDismiss={() => setAnswer(null)} />}
      {notice && (
        <NoticeCard
          key={notice.id}
          notice={notice}
          onDismiss={() => dispatch({ type: "remove", id: notice.id })}
        />
      )}
      {hovered && <LanguageSwitch />}
      {(state.phase !== "idle" || resting) && (
        <Pill state={state} levels={levels} error={notice?.level === "error"} privacy={privacy} />
      )}
    </div>
  );
}
