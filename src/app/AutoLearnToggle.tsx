import { useTranslation } from "react-i18next";
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
    <div className="card">
      <ToggleRow
        label={t("autoLearn.label")}
        description={t("autoLearn.description")}
        checked={settings.dictionary.autoLearn}
        onChange={(autoLearn) => void update({ dictionary: { autoLearn } })}
      />
    </div>
  );
}
