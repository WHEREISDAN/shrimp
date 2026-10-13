import type { Settings, SettingsPatch } from "../lib/ipc";
import { ToggleRow } from "./ToggleRow";

interface SoundSectionProps {
  settings: Settings;
  update: (patch: SettingsPatch) => Promise<void>;
}

/** Settings › Sound (SND-02..04). Muting and pausing exclude each other. */
export function SoundSection({ settings, update }: SoundSectionProps) {
  const { sounds, media } = settings;
  return (
    <section className="card stack" aria-labelledby="sound-heading">
      <h2 id="sound-heading">Sound</h2>
      <ToggleRow
        label="Dictation sounds"
        description="A sound when dictation starts, stops, locks hands-free or is cancelled."
        checked={sounds.dictation}
        onChange={(dictation) => void update({ sounds: { dictation } })}
      />
      <ToggleRow
        label="Notification sounds"
        description="A sound with warnings and errors."
        checked={sounds.notifications}
        onChange={(notifications) => void update({ sounds: { notifications } })}
      />
      <ToggleRow
        label="Mute audio while dictating"
        description="If something is playing when you start, your Mac stays muted until you finish."
        checked={media.mute}
        onChange={(mute) => void update({ media: mute ? { mute, pause: false } : { mute } })}
      />
      <ToggleRow
        label="Pause music while dictating"
        description="Pauses Music, Spotify and other players (never calls) and resumes them when you finish. Replaces muting."
        checked={media.pause}
        onChange={(pause) => void update({ media: pause ? { pause, mute: false } : { pause } })}
      />
    </section>
  );
}
