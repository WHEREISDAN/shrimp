import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { EVENTS, listHistory, type HistoryItem } from "../../lib/ipc";
import { useTauriEvent } from "../../lib/useTauriEvent";

interface PracticeStepProps {
  shortcut: string;
}

/** Practice dictations in a demo field, with what Shrimp wrote (ONB-06). */
export function PracticeStep({ shortcut }: PracticeStepProps) {
  const { t } = useTranslation();
  const [since] = useState(() => Date.now());
  const [latest, setLatest] = useState<HistoryItem | null>(null);
  const fieldRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => fieldRef.current?.focus(), []);
  const load = useCallback(() => {
    listHistory("")
      .then((list) => {
        const first = list.entries[0];
        if (first && first.createdAt >= since) {
          setLatest(first);
        }
      })
      .catch(() => undefined);
  }, [since]);
  useTauriEvent(EVENTS.history, load);

  return (
    <>
      <p>{t("onboarding.practiceBody", { shortcut })}</p>
      <ol className="intro-steps">
        <li>{t("onboarding.practice1")}</li>
        <li>{t("onboarding.practice2")}</li>
      </ol>
      <label className="field">
        <span className="card-title">{t("onboarding.practiceField")}</span>
        <textarea ref={fieldRef} rows={4} />
      </label>
      {latest && (
        <dl className="practice-result" aria-live="polite">
          <div>
            <dt>{t("onboarding.youSaid")}</dt>
            <dd>{latest.rawText}</dd>
          </div>
          <div>
            <dt>{t("onboarding.shrimpWrote")}</dt>
            <dd>{latest.finalText}</dd>
          </div>
        </dl>
      )}
    </>
  );
}
