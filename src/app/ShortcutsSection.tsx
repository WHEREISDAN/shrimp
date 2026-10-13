import { setShortcut, type Settings } from "../lib/ipc";
import { ShortcutRecorder } from "./ShortcutRecorder";

interface ShortcutsSectionProps {
  settings: Settings;
  onSaved: (settings: Settings) => void;
}

export function ShortcutsSection({ settings, onSaved }: ShortcutsSectionProps) {
  return (
    <section className="card stack" aria-labelledby="shortcut-settings-heading">
      <h2 id="shortcut-settings-heading">Shortcuts</h2>
      <ShortcutRecorder
        label="Press Enter"
        value={settings.shortcuts.pressEnter}
        onSave={async (shortcut) => onSaved(await setShortcut("pressEnter", shortcut))}
      />
      <p className="muted">
        Press Enter sends Enter to the app you are in. Hold Shift or another modifier with it to
        send Shift+Enter.
      </p>
    </section>
  );
}
