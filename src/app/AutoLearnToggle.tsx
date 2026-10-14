import { useTranslation } from "react-i18next";
import { Group } from "@/components/settings";
import { ToggleRow } from "./ToggleRow";
import { useSettings } from "./useSettings";

/** "Auto-add to dictionary" (DIC-07). */
export function AutoLearnToggle() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  if (!settings) {
    return null;
  }
  return (
    <Group>
      <ToggleRow
        label={t("autoLearn.label")}
        description={t("autoLearn.description")}
        checked={settings.dictionary.autoLearn}
        onChange={(autoLearn) => void update({ dictionary: { autoLearn } })}
      />
    </Group>
  );
}
