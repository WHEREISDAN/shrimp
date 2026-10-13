import { Trans, useTranslation } from "react-i18next";
import type { Settings, SettingsPatch } from "../lib/ipc";
import { ToggleRow } from "./ToggleRow";

interface VibeCodingSectionProps {
  settings: Settings;
  update: (patch: SettingsPatch) => Promise<void>;
}

export function VibeCodingSection({ settings, update }: VibeCodingSectionProps) {
  const { t } = useTranslation();
  return (
    <section className="card stack" aria-labelledby="vibe-coding-heading">
      <h2 id="vibe-coding-heading">{t("vibeCoding.title")}</h2>
      <ToggleRow
        label={t("vibeCoding.fileTagging")}
        description={t("vibeCoding.fileTaggingBody")}
        checked={settings.vibeCoding.fileTagging}
        onChange={(fileTagging) => void update({ vibeCoding: { fileTagging } })}
      />
      <div className="toggle-text">
        <span className="card-title">{t("vibeCoding.variables")}</span>
        <p className="muted">
          <Trans i18nKey="vibeCoding.variablesBody" components={{ code: <code /> }} />
        </p>
        <ol className="intro-steps muted">
          <li>{t("vibeCoding.step1")}</li>
          <li>{t("vibeCoding.step2")}</li>
          <li>{t("vibeCoding.step3")}</li>
        </ol>
      </div>
    </section>
  );
}
