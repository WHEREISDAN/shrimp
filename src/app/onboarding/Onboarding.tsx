import { useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
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
      body = <p>{t("onboarding.welcomeBody")}</p>;
      continueLabel = t("onboarding.getStarted");
      break;
    case "permissions":
      body = (
        <>
          <p>{t("onboarding.permissionsBody")}</p>
          <PermissionRows info={permissions} />
          <p className="muted">{t("onboarding.permissionsNote")}</p>
        </>
      );
      canContinue = granted;
      break;
    case "microphone":
      body = (
        <>
          <p>{t("onboarding.microphoneBody")}</p>
          <MicrophoneSection />
        </>
      );
      break;
    case "shortcut":
      body = (
        <>
          <p className="onboarding-shortcut">
            <kbd>{shortcut.talk}</kbd>
          </p>
          <p>{t("onboarding.shortcutHold", { shortcut: shortcut.talk })}</p>
          <p>
            {t("onboarding.shortcutHandsFree", {
              shortcut: shortcut.talk,
              handsFree: shortcut.handsFree,
            })}
          </p>
          <div className="onboarding-inline">
            <button type="button" onClick={() => setShortcutsOpen(true)}>
              {t("onboarding.changeShortcut")}
            </button>
          </div>
          {!IS_WINDOWS && (
            <p className="muted">
              {t("onboarding.globeKey")}{" "}
              <button
                type="button"
                className="link-button"
                onClick={() => void runNoticeAction("openKeyboardSettings").catch(() => undefined)}
              >
                {t("onboarding.openKeyboardSettings")}
              </button>
            </p>
          )}
        </>
      );
      break;
    case "languages":
      body = (
        <>
          <p>{t("onboarding.languagesBody")}</p>
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
        <ul className="intro-steps">
          <li>{t("onboarding.doneTip1", { shortcut: shortcut.talk })}</li>
          <li>{t("onboarding.doneTip2")}</li>
          <li>{t("onboarding.doneTip3")}</li>
        </ul>
      );
      continueLabel = t("onboarding.finish");
      break;
  }

  const index = STEPS.indexOf(step);
  return (
    <main className="onboarding" aria-labelledby="onboarding-title">
      <ol className="onboarding-progress" aria-label={t("onboarding.progress")}>
        {STEPS.map((id, position) => (
          <li
            key={id}
            className={position <= index ? "done" : undefined}
            aria-current={id === step ? "step" : undefined}
          >
            <span className="sr-only">{t(`onboarding.titles.${id}`)}</span>
          </li>
        ))}
      </ol>
      <section className="onboarding-card">
        <h1 key={step} id="onboarding-title" ref={focusOnMount} tabIndex={-1}>
          {t(`onboarding.titles.${step}`)}
        </h1>
        {body}
        <div className="onboarding-actions">
          {step !== "welcome" && (
            <button type="button" onClick={() => go(previousStep(step))}>
              {t("onboarding.back")}
            </button>
          )}
          {step === "practice" && (
            <button type="button" onClick={() => go("done")}>
              {t("onboarding.skip")}
            </button>
          )}
          <button
            type="button"
            className="primary"
            disabled={!canContinue}
            onClick={() => (step === "done" ? finish() : go(nextStep(step)))}
          >
            {continueLabel}
          </button>
        </div>
      </section>
      {shortcutsOpen && <ShortcutsDialog onClose={() => setShortcutsOpen(false)} />}
    </main>
  );
}
