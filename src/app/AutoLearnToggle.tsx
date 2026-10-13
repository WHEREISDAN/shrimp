import { ToggleRow } from "./ToggleRow";
import { useSettings } from "./useSettings";

/** "Auto-add to dictionary" (DIC-07). */
export function AutoLearnToggle() {
  const { settings, update } = useSettings();
  if (!settings) {
    return null;
  }
  return (
    <div className="card">
      <ToggleRow
        label="Auto-add to dictionary"
        description="When you correct a word right after dictating it, Shrimp adds the new spelling. Paths, links, code, numbers, everyday words and words you delete here are never added."
        checked={settings.dictionary.autoLearn}
        onChange={(autoLearn) => void update({ dictionary: { autoLearn } })}
      />
    </div>
  );
}
