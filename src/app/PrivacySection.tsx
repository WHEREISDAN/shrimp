import type { Settings, SettingsPatch } from "../lib/ipc";
import { ToggleRow } from "./ToggleRow";

interface PrivacySectionProps {
  settings: Settings;
  update: (patch: SettingsPatch) => Promise<void>;
}

export function PrivacySection({ settings, update }: PrivacySectionProps) {
  return (
    <section className="card stack" aria-labelledby="privacy-heading">
      <h2 id="privacy-heading">Data and privacy</h2>
      <ToggleRow
        label="Context awareness"
        description="Shrimp reads the text around your cursor and the names on screen, so names are spelled right and the style fits the app. When off, nothing from your screen reaches the speech engine or the AI provider; casing and spacing still adapt to the text around the cursor, on this Mac."
        checked={settings.privacy.contextAwareness}
        onChange={(contextAwareness) => void update({ privacy: { contextAwareness } })}
      />
    </section>
  );
}
