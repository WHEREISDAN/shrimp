import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Textarea } from "@/components/ui/textarea";
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
      <p className="text-muted-foreground">{t("onboarding.practiceBody", { shortcut })}</p>
      <ol className="m-0 flex list-none flex-col gap-2 p-0">
        {[t("onboarding.practice1"), t("onboarding.practice2")].map((step, index) => (
          <li key={index} className="flex items-start gap-3">
            <span
              aria-hidden="true"
              className="grid size-5 shrink-0 place-items-center rounded-full bg-brand/12 text-xs font-semibold text-brand-strong tabular-nums"
            >
              {index + 1}
            </span>
            <span>{step}</span>
          </li>
        ))}
      </ol>
      <label className="field flex flex-col gap-1.5">
        <span className="text-sm font-medium">{t("onboarding.practiceField")}</span>
        <Textarea ref={fieldRef} rows={4} className="min-h-24 text-base" />
      </label>
      {latest && (
        <dl
          className="practice-result m-0 grid gap-3 rounded-xl border bg-card p-4 shadow-xs"
          aria-live="polite"
        >
          <div>
            <dt className="text-xs font-medium text-muted-foreground">{t("onboarding.youSaid")}</dt>
            <dd className="m-0 mt-0.5">{latest.rawText}</dd>
          </div>
          <div>
            <dt className="text-xs font-medium text-muted-foreground">
              {t("onboarding.shrimpWrote")}
            </dt>
            <dd className="m-0 mt-0.5">{latest.finalText}</dd>
          </div>
        </dl>
      )}
    </>
  );
}
