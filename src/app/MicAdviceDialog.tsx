import { Check, ChevronRight, Mic } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { Dialog, DialogFooter } from "./Dialog";
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
    const position = STEPS.indexOf(step);
    return (
      <Dialog title={t("micAdvice.title")} onClose={onClose}>
        <div className="flex flex-col gap-2">
          <p className="text-sm text-muted-foreground tabular-nums">
            {t("micAdvice.progress", { current: position + 1, total: STEPS.length })}
          </p>
          <div aria-hidden="true" className="flex gap-1">
            {STEPS.map((key, index) => (
              <span
                key={key}
                className={cn(
                  "h-1 flex-1 rounded-full transition-colors duration-200",
                  index <= position ? "bg-brand" : "bg-muted",
                )}
              />
            ))}
          </div>
        </div>
        <fieldset className="m-0 flex min-w-0 flex-col border-0 p-0">
          <legend className="mb-2.5 px-1 font-medium">{t(question.label)}</legend>
          <div className="flex flex-col divide-y divide-separator rounded-xl border bg-card shadow-xs">
            {question.options.map((option) => (
              <button
                key={option.value}
                type="button"
                className="flex min-h-(--row-min-h) w-full items-center justify-between gap-3 px-4 py-2.5 text-left transition-colors duration-100 outline-none first:rounded-t-xl last:rounded-b-xl hover:bg-muted/50 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring/70 dark:hover:bg-white/[0.04]"
                onClick={() => setAnswers({ ...answers, [step]: option.value })}
              >
                {t(option.label)}
                <ChevronRight
                  aria-hidden="true"
                  className="size-4 shrink-0 text-muted-foreground"
                />
              </button>
            ))}
          </div>
        </fieldset>
        <DialogFooter>
          <Button onClick={onClose}>{t("common.cancel")}</Button>
        </DialogFooter>
      </Dialog>
    );
  }

  const advice = recommend(answers as MicAnswers);
  return (
    <Dialog title={t("micAdvice.title")} onClose={onClose}>
      <div className="flex items-start gap-3">
        <span
          aria-hidden="true"
          className="grid size-9 shrink-0 place-items-center rounded-full bg-brand/12 text-brand"
        >
          <Mic className="size-4" />
        </span>
        <div className="flex min-w-0 flex-col gap-1 pt-0.5">
          <p className="font-medium">{t(advice.title)}</p>
          <p className="text-muted-foreground">{t(advice.why)}</p>
        </div>
      </div>
      <ul className="flex flex-col divide-y divide-separator rounded-xl border bg-card shadow-xs">
        {advice.tips.map((tip) => (
          <li key={tip} className="flex items-start gap-3 px-4 py-2.5">
            <Check aria-hidden="true" className="mt-px size-4 shrink-0 text-brand" />
            <span>{t(tip)}</span>
          </li>
        ))}
      </ul>
      <DialogFooter>
        <Button onClick={() => setAnswers({})}>{t("micAdvice.startOver")}</Button>
        <Button variant="default" onClick={onClose}>
          {t("dialog.done")}
        </Button>
      </DialogFooter>
    </Dialog>
  );
}
