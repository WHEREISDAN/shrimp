import { X } from "lucide-react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { diffWords } from "../lib/diff";
import {
  transformAction,
  type QuickTransform,
  type TransformAction,
  type TransformView,
} from "../lib/ipc";

/** Re-transforms of the original text, in Wispr Flow's order (TRF-04). */
const QUICK: QuickTransform[] = ["promptEngineer", "turnToList", "translateToEnglish", "empathize"];

interface ChangesCardProps {
  view: TransformView;
  onClose: () => void;
}

/** What a transform changed, with Copy, Retry, re-transforms, Accept edits and Undo (TRF-04). */
export function ChangesCard({ view, onClose }: ChangesCardProps) {
  const { t } = useTranslation();
  const parts = useMemo(() => diffWords(view.original, view.result), [view]);
  const act = (action: TransformAction, close: boolean) => () => {
    void transformAction(action).catch(() => undefined);
    if (close) {
      onClose();
    }
  };
  return (
    <section className="answer changes" aria-label={t("changes.region", { name: view.name })}>
      <header className="answer-head">
        <p className="answer-question">{view.name}</p>
        <button
          type="button"
          className="notice-close"
          aria-label={t("common.close")}
          onClick={onClose}
        >
          <X aria-hidden="true" strokeWidth={2} />
        </button>
      </header>
      <div className="answer-text diff">
        {parts.map((part, index) =>
          part.kind === "same" ? (
            <span key={index}>{part.text}</span>
          ) : part.kind === "added" ? (
            <ins key={index}>{part.text}</ins>
          ) : (
            <del key={index}>{part.text}</del>
          ),
        )}
      </div>
      <div className="quick-transforms" role="group" aria-label={t("changes.quick.label")}>
        {QUICK.map((quick) => (
          <button
            key={quick}
            type="button"
            className="quick-transform"
            onClick={act({ retransform: quick }, false)}
          >
            {t(`changes.quick.${quick}`)}
          </button>
        ))}
      </div>
      <div className="answer-actions">
        <button type="button" className="notice-action" onClick={act("copy", false)}>
          {t("common.copy")}
        </button>
        <button type="button" className="notice-action" onClick={act("retry", false)}>
          {t("changes.retry")}
        </button>
        <button type="button" className="notice-action" onClick={act("undo", true)}>
          {t("changes.undo")}
        </button>
        <button type="button" className="notice-action primary" onClick={onClose}>
          {t("changes.accept")}
        </button>
      </div>
    </section>
  );
}
