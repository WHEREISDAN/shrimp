import { useEffect, useRef } from "react";
import type { Settings, SettingsPatch } from "../lib/ipc";
import { ToggleRow } from "./ToggleRow";

interface ExperimentalSectionProps {
  settings: Settings;
  /** The Command Mode shortcut, e.g. "fn ⌃". */
  commandShortcut: string;
  /** Increments when "Go to settings" should bring this section into view. */
  reveal: number;
  update: (patch: SettingsPatch) => Promise<void>;
  onCommandModeOn: () => void;
}

export function ExperimentalSection({
  settings,
  commandShortcut,
  reveal,
  update,
  onCommandModeOn,
}: ExperimentalSectionProps) {
  const headingRef = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    if (reveal === 0) {
      return;
    }
    const frame = requestAnimationFrame(() => {
      headingRef.current?.scrollIntoView({ block: "start" });
      headingRef.current?.focus();
    });
    return () => cancelAnimationFrame(frame);
  }, [reveal]);

  return (
    <section className="card stack" id="experimental" aria-labelledby="experimental-heading">
      <h2 id="experimental-heading" ref={headingRef} tabIndex={-1}>
        Experimental
      </h2>
      <ToggleRow
        label="Command Mode"
        description={`Select text, hold ${commandShortcut} and say what to change, like “make this more formal” or “translate to Spanish”. With nothing selected, ask a question.`}
        checked={settings.experimental.commandMode}
        onChange={(commandMode) => {
          void update({ experimental: { commandMode } });
          if (commandMode && !settings.experimental.commandModeIntroSeen) {
            onCommandModeOn();
          }
        }}
      />
      <ToggleRow
        label="“Press enter” voice command"
        description="End a dictation with “press enter” to send it: Shrimp leaves the words out and presses Enter after pasting."
        checked={settings.experimental.pressEnterCommand}
        onChange={(pressEnterCommand) => void update({ experimental: { pressEnterCommand } })}
      />
    </section>
  );
}
