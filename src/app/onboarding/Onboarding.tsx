import { CircleCheck } from "lucide-react";
import { useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { AppIcon } from "@/components/app-icon";
import { DragBand } from "@/components/drag-band";
import { Button } from "@/components/ui/button";
import { Kbd } from "@/components/ui/kbd";
import { cn } from "@/lib/utils";
import type { OnboardingStep, Settings, SettingsPatch } from "../../lib/ipc";
import { ShortcutsDialog } from "../ShortcutsDialog";
import { EngineStep } from "./EngineStep";
import { PracticeStep } from "./PracticeStep";
import { PermissionRows } from "./PermissionRows";
import { nextStep, previousStep, STEPS } from "./steps";
import { usePermissions } from "./usePermissions";
import { LanguagesSection } from "../LanguagesSection";
import { MicrophoneSection } from "../MicrophoneSection";
import { usePushToTalk } from "./usePushToTalk";
import { runNoticeAction } from "../../lib/ipc";
import { IS_WINDOWS } from "../../lib/platform";

/** Each step's title takes focus, so screen readers announce the new step. */
const focusOnMount = (node: HTMLHeadingElement | null) => node?.focus();

interface OnboardingProps {
  settings: Settings;
  update: (patch: SettingsPatch) => Promise<void>;
}

/**
 * First launch (ONB-01..07): permissions, the microphone, the shortcut,
 * languages, the speech engine and practice. Each step is saved, so a
 * restart resumes where it stopped; dictation shortcuts work from Practice.
 */
export function Onboarding({ settings, update }: OnboardingProps) {
  const { t } = useTranslation();
  const step = settings.onboarding.step;
  const permissions = usePermissions();
  const shortcut = usePushToTalk();
  const [shortcutsOpen, setShortcutsOpen] = useState(false);

  const go = (next: OnboardingStep) => void update({ onboarding: { step: next } });
  const finish = () => void update({ onboarding: { step: "done", completed: true } });
  const granted = permissions?.microphone === "granted" && permissions.accessibility === "granted";

  let body: ReactNode;
  let canContinue = true;
  let continueLabel: string = t("onboarding.continue");
  switch (step) {
    case "welcome":
      body = <p className="text-muted-foreground">{t("onboarding.welcomeBody")}</p>;
      continueLabel = t("onboarding.getStarted");
      break;
    case "permissions":
      body = (
        <>
          <p className="text-muted-foreground">{t("onboarding.permissionsBody")}</p>
          <PermissionRows info={permissions} />
          <p className="text-sm text-muted-foreground">{t("onboarding.permissionsNote")}</p>
        </>
      );
      canContinue = granted;
      break;
    case "microphone":
      body = (
        <>
          <p className="text-muted-foreground">{t("onboarding.microphoneBody")}</p>
          <MicrophoneSection />
        </>
      );
      break;
    case "shortcut":
      body = (
        <>
          <div className="onboarding-shortcut flex justify-center rounded-xl border bg-card py-8 shadow-xs">
            <Kbd className="h-14 min-w-20 rounded-xl px-5 text-3xl font-semibold text-foreground shadow-[0_2px_0_var(--border)]">
              {shortcut.talk}
            </Kbd>
          </div>
          <p>{t("onboarding.shortcutHold", { shortcut: shortcut.talk })}</p>
          <p className="text-muted-foreground">
            {t("onboarding.shortcutHandsFree", {
              shortcut: shortcut.talk,
              handsFree: shortcut.handsFree,
            })}
          </p>
          <div>
            <Button onClick={() => setShortcutsOpen(true)}>{t("onboarding.changeShortcut")}</Button>
          </div>
          {!IS_WINDOWS && (
            <p className="text-sm text-muted-foreground">
              {t("onboarding.globeKey")}{" "}
              <Button
                variant="link"
                className="text-sm"
                onClick={() => void runNoticeAction("openKeyboardSettings").catch(() => undefined)}
              >
                {t("onboarding.openKeyboardSettings")}
              </Button>
            </p>
          )}
        </>
      );
      break;
    case "languages":
      body = (
        <>
          <p className="text-muted-foreground">{t("onboarding.languagesBody")}</p>
          <LanguagesSection />
        </>
      );
      break;
    case "engine":
      body = <EngineStep settings={settings} update={update} />;
      break;
    case "practice":
      body = <PracticeStep shortcut={shortcut.talk} />;
      continueLabel = t("onboarding.continue");
      break;
    case "done":
      body = (
        <ul className="m-0 flex list-none flex-col divide-y divide-separator rounded-xl border bg-card p-0 shadow-xs">
          {[
            t("onboarding.doneTip1", { shortcut: shortcut.talk }),
            t("onboarding.doneTip2"),
            t("onboarding.doneTip3"),
          ].map((tip) => (
            <li key={tip} className="flex items-start gap-3 px-4 py-3">
              <CircleCheck aria-hidden="true" className="mt-0.5 size-4 shrink-0 text-brand" />
              <span>{tip}</span>
            </li>
          ))}
        </ul>
      );
      continueLabel = t("onboarding.finish");
      break;
  }

  const index = STEPS.indexOf(step);
  return (
    <main
      className="onboarding flex h-full flex-col bg-background"
      aria-labelledby="onboarding-title"
    >
      <DragBand />
      <div className="flex min-h-0 flex-1 flex-col items-center overflow-y-auto px-8 pt-16 pb-8">
        <div className="my-auto flex w-full max-w-[32rem] flex-col gap-6">
          <div className="flex items-center justify-between gap-4">
            <AppIcon className="-m-1.5 size-16" />
            <ol
              className="onboarding-progress m-0 flex list-none items-center gap-1.5 p-0"
              aria-label={t("onboarding.progress")}
            >
              {STEPS.map((id, position) => (
                <li
                  key={id}
                  className={cn(
                    "h-1.5 w-4 rounded-full bg-foreground/12 transition-[width,background-color] duration-300 ease-spring",
                    position <= index && "done bg-brand",
                    id === step && "w-7",
                  )}
                  aria-current={id === step ? "step" : undefined}
                >
                  <span className="sr-only">{t(`onboarding.titles.${id}`)}</span>
                </li>
              ))}
            </ol>
          </div>
          <section
            key={step}
            className="onboarding-card flex animate-pop-in flex-col gap-4 [&>p]:m-0"
          >
            <h1
              id="onboarding-title"
              ref={focusOnMount}
              tabIndex={-1}
              className="font-display text-3xl font-bold tracking-[-0.015em] outline-none"
            >
              {t(`onboarding.titles.${step}`)}
            </h1>
            {body}
          </section>
        </div>
      </div>
      <footer className="shrink-0 border-t bg-background/85 px-8 py-4 backdrop-blur-xl">
        <div className="onboarding-actions mx-auto flex w-full max-w-[32rem] flex-wrap items-center justify-end gap-2">
          {step !== "welcome" && (
            <Button
              size="lg"
              variant="ghost"
              className="mr-auto"
              onClick={() => go(previousStep(step))}
            >
              {t("onboarding.back")}
            </Button>
          )}
          {step === "practice" && (
            <Button size="lg" onClick={() => go("done")}>
              {t("onboarding.skip")}
            </Button>
          )}
          <Button
            size="lg"
            variant="default"
            disabled={!canContinue}
            onClick={() => (step === "done" ? finish() : go(nextStep(step)))}
          >
            {continueLabel}
          </Button>
        </div>
      </footer>
      {shortcutsOpen && <ShortcutsDialog onClose={() => setShortcutsOpen(false)} />}
    </main>
  );
}
