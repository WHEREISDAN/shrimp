import { useTranslation } from "react-i18next";
import type { Settings, SettingsPatch } from "../lib/ipc";
import { DeleteTranscriptsSetting } from "./DeleteTranscriptsSetting";
import { HistoryStorageSetting } from "./HistoryStorageSetting";
import { ToggleRow } from "./ToggleRow";

interface PrivacySectionProps {
  settings: Settings;
  update: (patch: SettingsPatch) => Promise<void>;
}

export function PrivacySection({ settings, update }: PrivacySectionProps) {
  const { t } = useTranslation();
  return (
    <section className="card stack" aria-labelledby="privacy-heading">
      <h2 id="privacy-heading">{t("settings.privacy")}</h2>
      <ToggleRow
        label={t("privacy.privacyMode")}
        description={t("privacy.privacyModeBody")}
        checked={settings.privacy.privacyMode}
        onChange={(privacyMode) => void update({ privacy: { privacyMode } })}
      />
      <ToggleRow
        label={t("contextAwareness.title")}
        description={t("contextAwareness.body")}
        checked={settings.privacy.contextAwareness}
        onChange={(contextAwareness) => void update({ privacy: { contextAwareness } })}
      />
      <HistoryStorageSetting value={settings.privacy.history} />
      <DeleteTranscriptsSetting disabled={settings.privacy.history === "never"} />
    </section>
  );
}
