import type { ReactNode } from "react";
import { Trans, useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Kbd } from "@/components/ui/kbd";
import { Dialog, DialogFooter } from "./Dialog";

interface CommandModeIntroProps {
  shortcut: string;
  onDone: () => void;
  onDisable: () => void;
}

/** One numbered step; the list itself gives screen readers the numbers. */
function Step({ number, children }: { number: number; children: ReactNode }) {
  return (
    <li className="flex items-start gap-3">
      <span
        aria-hidden="true"
        className="flex size-5 shrink-0 items-center justify-center rounded-full bg-muted text-xs font-semibold text-muted-foreground tabular-nums"
      >
        {number}
      </span>
      <span className="min-w-0 flex-1 pt-px">{children}</span>
    </li>
  );
}

/** Shown once, when Command Mode is first turned on (CMD-06). */
export function CommandModeIntro({ shortcut, onDone, onDisable }: CommandModeIntroProps) {
  const { t } = useTranslation();
  return (
    <Dialog title={t("commandModeIntro.title")} onClose={onDone}>
      <ol className="flex flex-col gap-3">
        <Step number={1}>{t("commandModeIntro.step1")}</Step>
        <Step number={2}>
          <Trans
            i18nKey="commandModeIntro.step2"
            values={{ shortcut }}
            components={{ kbd: <Kbd /> }}
          />
        </Step>
        <Step number={3}>{t("commandModeIntro.step3")}</Step>
      </ol>
      <p className="text-sm text-muted-foreground">{t("commandModeIntro.body")}</p>
      <DialogFooter>
        <Button onClick={onDisable}>{t("commandModeIntro.disable")}</Button>
        <Button variant="default" onClick={onDone} autoFocus>
          {t("commandModeIntro.gotIt")}
        </Button>
      </DialogFooter>
    </Dialog>
  );
}
