import { useTranslation } from "react-i18next";
import { Group, Section } from "@/components/settings";
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
    <>
      <Section headingId="privacy-heading" title={t("settings.privacy")}>
        <Group>
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
        </Group>
      </Section>
      <HistoryStorageSetting value={settings.privacy.history} />
      <DeleteTranscriptsSetting disabled={settings.privacy.history === "never"} />
    </>
  );
}
