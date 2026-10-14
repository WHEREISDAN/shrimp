import { X } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { runNoticeAction, type Answer } from "../lib/ipc";

interface AnswerCardProps {
  answer: Answer;
  onDismiss: () => void;
}

/** A Command Mode answer, shown instead of pasted (CMD-03). */
export function AnswerCard({ answer, onDismiss }: AnswerCardProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  return (
    <section className="answer" aria-label={t("answer.region")}>
      <header className="answer-head">
        <p className="answer-question">{answer.question}</p>
        <button
          type="button"
          className="notice-close"
          aria-label={t("answer.dismiss")}
          onClick={onDismiss}
        >
          <X aria-hidden="true" strokeWidth={2} />
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
          {copied ? t("answer.copied") : t("common.copy")}
        </button>
      </div>
    </section>
  );
}
