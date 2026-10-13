import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
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
  const { t } = useTranslation();
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
        {t("experimental.title")}
      </h2>
      <ToggleRow
        label={t("experimental.commandMode")}
        description={t("experimental.commandModeBody", { shortcut: commandShortcut })}
        checked={settings.experimental.commandMode}
        onChange={(commandMode) => {
          void update({ experimental: { commandMode } });
          if (commandMode && !settings.experimental.commandModeIntroSeen) {
            onCommandModeOn();
          }
        }}
      />
      <ToggleRow
        label={t("experimental.pressEnter")}
        description={t("experimental.pressEnterBody")}
        checked={settings.experimental.pressEnterCommand}
        onChange={(pressEnterCommand) => void update({ experimental: { pressEnterCommand } })}
      />
    </section>
  );
}
