import { useTranslation } from "react-i18next";
import { Group, Section } from "@/components/settings";
import type { Settings, SettingsPatch } from "../lib/ipc";
import { ToggleRow } from "./ToggleRow";

interface SoundSectionProps {
  settings: Settings;
  update: (patch: SettingsPatch) => Promise<void>;
}

/** Settings › Sound (SND-02..04). Muting and pausing exclude each other. */
export function SoundSection({ settings, update }: SoundSectionProps) {
  const { t } = useTranslation();
  const { sounds, media } = settings;
  return (
    <Section headingId="sound-heading" title={t("sound.title")}>
      <Group>
        <ToggleRow
          label={t("sound.dictation")}
          description={t("sound.dictationBody")}
          checked={sounds.dictation}
          onChange={(dictation) => void update({ sounds: { dictation } })}
        />
        <ToggleRow
          label={t("sound.notifications")}
          description={t("sound.notificationsBody")}
          checked={sounds.notifications}
          onChange={(notifications) => void update({ sounds: { notifications } })}
        />
        <ToggleRow
          label={t("sound.mute")}
          description={t("sound.muteBody")}
          checked={media.mute}
          onChange={(mute) => void update({ media: mute ? { mute, pause: false } : { mute } })}
        />
        <ToggleRow
          label={t("sound.pause")}
          description={t("sound.pauseBody")}
          checked={media.pause}
          onChange={(pause) => void update({ media: pause ? { pause, mute: false } : { pause } })}
        />
      </Group>
    </Section>
  );
}
