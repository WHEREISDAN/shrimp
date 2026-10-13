import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Dialog } from "./Dialog";
import { QUESTIONS, recommend, type MicAnswers } from "./micAdvice";

type Step = keyof typeof QUESTIONS;
const STEPS: Step[] = ["place", "computer", "headphones"];

interface MicAdviceDialogProps {
  onClose: () => void;
}

/** "Find the right mic for you" (MIC-10): three questions, then a recommendation. */
export function MicAdviceDialog({ onClose }: MicAdviceDialogProps) {
  const { t } = useTranslation();
  const [answers, setAnswers] = useState<Partial<MicAnswers>>({});
  const step = STEPS.find((key) => answers[key] === undefined);

  if (step) {
    const question = QUESTIONS[step];
    return (
      <Dialog title={t("micAdvice.title")} onClose={onClose}>
        <p className="muted">
          {t("micAdvice.progress", { current: STEPS.indexOf(step) + 1, total: STEPS.length })}
        </p>
        <fieldset className="plain-fieldset stack">
          <legend className="card-title">{t(question.label)}</legend>
          {question.options.map((option) => (
            <button
              key={option.value}
              type="button"
              className="radio-card"
              onClick={() => setAnswers({ ...answers, [step]: option.value })}
            >
              {t(option.label)}
            </button>
          ))}
        </fieldset>
        <div className="dialog-actions">
          <button type="button" onClick={onClose}>
            {t("common.cancel")}
          </button>
        </div>
      </Dialog>
    );
  }

  const advice = recommend(answers as MicAnswers);
  return (
    <Dialog title={t("micAdvice.title")} onClose={onClose}>
      <p className="card-title">{t(advice.title)}</p>
      <p>{t(advice.why)}</p>
      <ul className="intro-steps">
        {advice.tips.map((tip) => (
          <li key={tip}>{t(tip)}</li>
        ))}
      </ul>
      <div className="dialog-actions">
        <button type="button" onClick={() => setAnswers({})}>
          {t("micAdvice.startOver")}
        </button>
        <button type="button" className="primary" onClick={onClose}>
          {t("dialog.done")}
        </button>
      </div>
    </Dialog>
  );
}
