import { useState } from "react";
import { runNoticeAction, type Answer } from "../lib/ipc";

interface AnswerCardProps {
  answer: Answer;
  onDismiss: () => void;
}

/** A Command Mode answer, shown instead of pasted (CMD-03). */
export function AnswerCard({ answer, onDismiss }: AnswerCardProps) {
  const [copied, setCopied] = useState(false);
  return (
    <section className="answer" aria-label="Command Mode answer">
      <header className="answer-head">
        <p className="answer-question">{answer.question}</p>
        <button type="button" className="notice-close" aria-label="Dismiss" onClick={onDismiss}>
          ×
        </button>
      </header>
      <div className="answer-text">{answer.text}</div>
      <div className="answer-actions">
        <button
          type="button"
          className="notice-action"
          onClick={() => {
            // The answer is the last transcript, so this copies it.
            void runNoticeAction("copyLastTranscript").catch(() => undefined);
            setCopied(true);
          }}
        >
          {copied ? "Copied ✓" : "Copy"}
        </button>
      </div>
    </section>
  );
}
