import { useCallback, useEffect, useRef, useState } from "react";
import { EVENTS, getShortcuts, type ShortcutList } from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";

interface ShortcutsCardProps {
  onOpenShortcuts: () => void;
  /** Increments each time the card should scroll into view and take focus. */
  reveal?: number;
}

export function ShortcutsCard({ onOpenShortcuts, reveal = 0 }: ShortcutsCardProps) {
  const [list, setList] = useState<ShortcutList | null>(null);
  const headingRef = useRef<HTMLHeadingElement>(null);

  const load = useCallback(() => {
    getShortcuts()
      .then(setList)
      .catch(() => undefined);
  }, []);
  useEffect(load, [load]);
  useTauriEvent(EVENTS.settings, load);

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
      <div className="row">
        <h2 id="shortcuts-heading" ref={headingRef} tabIndex={-1}>
          Shortcuts
        </h2>
        <button type="button" onClick={onOpenShortcuts}>
          Change
        </button>
      </div>
      <ul className="rows">
        {list?.rows
          .filter((row) => row.shortcuts.length > 0)
          .map((row) => (
            <li key={row.name} className="row">
              <span>{row.name}</span>
              <kbd>{row.shortcuts.join(" / ")}</kbd>
            </li>
          ))}
      </ul>
      <p className="muted">
        Double-tap push to talk to keep recording hands-free. Esc cancels while you dictate.
      </p>
    </section>
  );
}
