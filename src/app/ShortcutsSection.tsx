import { useCallback, useEffect, useState } from "react";
import { EVENTS, getShortcuts, type ShortcutList } from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";

interface ShortcutsSectionProps {
  onOpenShortcuts: () => void;
}

/** Settings › Shortcuts: what push to talk is, and the way into the Shortcuts dialog. */
export function ShortcutsSection({ onOpenShortcuts }: ShortcutsSectionProps) {
  const [list, setList] = useState<ShortcutList | null>(null);
  const load = useCallback(() => {
    getShortcuts()
      .then(setList)
      .catch(() => undefined);
  }, []);
  useEffect(load, [load]);
  useTauriEvent(EVENTS.settings, load);
  const talk = list?.rows.find((row) => row.action === "pushToTalk");

  return (
    <section className="card stack" aria-labelledby="shortcut-settings-heading">
      <h2 id="shortcut-settings-heading">Shortcuts</h2>
      <div className="row">
        <span>Push to talk {talk && <kbd>{talk.shortcuts.join(" / ")}</kbd>}</span>
        <button type="button" onClick={onOpenShortcuts}>
          Change
        </button>
      </div>
      <p className="muted">
        Each action takes up to four shortcuts, including mouse buttons. Press Enter sends Enter to
        the app you are in; hold Shift or another modifier with it to send Shift+Enter.
      </p>
    </section>
  );
}
