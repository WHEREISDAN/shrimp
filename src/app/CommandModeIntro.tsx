import { Trans, useTranslation } from "react-i18next";
import { Dialog } from "./Dialog";

interface CommandModeIntroProps {
  shortcut: string;
  onDone: () => void;
  onDisable: () => void;
}

/** Shown once, when Command Mode is first turned on (CMD-06). */
export function CommandModeIntro({ shortcut, onDone, onDisable }: CommandModeIntroProps) {
  const { t } = useTranslation();
  return (
    <Dialog title={t("commandModeIntro.title")} onClose={onDone}>
      <ol className="intro-steps">
        <li>{t("commandModeIntro.step1")}</li>
        <li>
          <Trans
            i18nKey="commandModeIntro.step2"
            values={{ shortcut }}
            components={{ kbd: <kbd /> }}
          />
        </li>
        <li>{t("commandModeIntro.step3")}</li>
      </ol>
      <p className="muted">{t("commandModeIntro.body")}</p>
      <div className="dialog-actions">
        <button type="button" onClick={onDisable}>
          {t("commandModeIntro.disable")}
        </button>
        <button type="button" className="primary" onClick={onDone} autoFocus>
          {t("commandModeIntro.gotIt")}
        </button>
      </div>
    </Dialog>
  );
}
