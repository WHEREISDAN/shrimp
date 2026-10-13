import { useMemo } from "react";
import { diffWords } from "../lib/diff";
import { transformAction, type TransformAction, type TransformView } from "../lib/ipc";

interface ChangesCardProps {
  view: TransformView;
  onClose: () => void;
}

/** What a transform changed, with Copy, Retry, Accept edits and Undo (TRF-04). */
export function ChangesCard({ view, onClose }: ChangesCardProps) {
  const parts = useMemo(() => diffWords(view.original, view.result), [view]);
  const act = (action: TransformAction, close: boolean) => () => {
    void transformAction(action).catch(() => undefined);
    if (close) {
      onClose();
    }
  };
  return (
    <section className="answer changes" aria-label={`${view.name} changes`}>
      <header className="answer-head">
        <p className="answer-question">{view.name}</p>
        <button type="button" className="notice-close" aria-label="Close" onClick={onClose}>
          ×
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
      <div className="answer-actions">
        <button type="button" className="notice-action" onClick={act("copy", false)}>
          Copy
        </button>
        <button type="button" className="notice-action" onClick={act("retry", false)}>
          Retry
        </button>
        <button type="button" className="notice-action" onClick={act("undo", true)}>
          Undo
        </button>
        <button type="button" className="notice-action primary" onClick={onClose}>
          Accept edits
        </button>
      </div>
    </section>
  );
}
