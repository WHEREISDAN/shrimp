import { useEffect, useRef, useState } from "react";
import { getShortcuts, type ShortcutInfo } from "../lib/ipc";

interface ShortcutsCardProps {
  /** Increments each time the card should scroll into view and take focus. */
  reveal: number;
}

export function ShortcutsCard({ reveal }: ShortcutsCardProps) {
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

  useEffect(() => {
    if (reveal === 0) {
      return;
    }
    // After the page switch has painted, so the card is laid out before it scrolls.
    const frame = requestAnimationFrame(() => {
      headingRef.current?.scrollIntoView({ block: "start" });
      headingRef.current?.focus();
    });
    return () => cancelAnimationFrame(frame);
  }, [reveal]);

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
