import { useEffect, useRef, useState } from "react";
import { EVENTS, getShortcuts, type ShortcutInfo } from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";

export function ShortcutsCard() {
  const [shortcuts, setShortcuts] = useState<ShortcutInfo[]>([]);
  const headingRef = useRef<HTMLHeadingElement>(null);

  useEffect(() => {
    let cancelled = false;
    getShortcuts()
      .then((list) => !cancelled && setShortcuts(list))
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);

  useTauriEvent<string>(EVENTS.navigate, (section) => {
    if (section === "shortcuts") {
      headingRef.current?.scrollIntoView({ block: "start" });
      headingRef.current?.focus();
    }
  });

  return (
    <section className="card" id="shortcuts" aria-labelledby="shortcuts-heading">
      <h2 id="shortcuts-heading" ref={headingRef} tabIndex={-1}>
        Shortcuts
      </h2>
      <ul className="rows">
        {shortcuts.map((shortcut) => (
          <li key={shortcut.action} className="row">
            <span>{shortcut.name}</span>
            <kbd>{shortcut.shortcut}</kbd>
          </li>
        ))}
      </ul>
      <p className="muted">
        Double-tap push to talk to keep recording hands-free. Esc cancels while you dictate.
      </p>
    </section>
  );
}
