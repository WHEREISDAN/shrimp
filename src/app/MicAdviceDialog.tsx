import { useState } from "react";
import { Dialog } from "./Dialog";
import { QUESTIONS, recommend, type MicAnswers } from "./micAdvice";

type Step = keyof typeof QUESTIONS;
const STEPS: Step[] = ["place", "computer", "headphones"];

interface MicAdviceDialogProps {
  onClose: () => void;
}

/** "Find the right mic for you" (MIC-10): three questions, then a recommendation. */
export function MicAdviceDialog({ onClose }: MicAdviceDialogProps) {
  const [answers, setAnswers] = useState<Partial<MicAnswers>>({});
  const step = STEPS.find((key) => answers[key] === undefined);

  if (step) {
    const question = QUESTIONS[step];
    return (
      <Dialog title="Find the right mic for you" onClose={onClose}>
        <p className="muted">
          Question {STEPS.indexOf(step) + 1} of {STEPS.length}
        </p>
        <fieldset className="plain-fieldset stack">
          <legend className="card-title">{question.label}</legend>
          {question.options.map((option) => (
            <button
              key={option.value}
              type="button"
              className="radio-card"
              onClick={() => setAnswers({ ...answers, [step]: option.value })}
            >
              {option.label}
            </button>
          ))}
        </fieldset>
        <div className="dialog-actions">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
        </div>
      </Dialog>
    );
  }

  const advice = recommend(answers as MicAnswers);
  return (
    <Dialog title="Find the right mic for you" onClose={onClose}>
      <p className="card-title">{advice.title}</p>
      <p>{advice.why}</p>
      <ul className="intro-steps">
        {advice.tips.map((tip) => (
          <li key={tip}>{tip}</li>
        ))}
      </ul>
      <div className="dialog-actions">
        <button type="button" onClick={() => setAnswers({})}>
          Start over
        </button>
        <button type="button" className="primary" onClick={onClose}>
          Done
        </button>
      </div>
    </Dialog>
  );
}
