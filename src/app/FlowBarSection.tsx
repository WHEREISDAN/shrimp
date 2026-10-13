import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { Settings, SettingsPatch } from "../lib/ipc";
import { ToggleRow } from "./ToggleRow";

interface FlowBarSectionProps {
  settings: Settings;
  update: (patch: SettingsPatch) => Promise<void>;
}

/** Settings › Flow Bar (BAR-04, BAR-05). */
export function FlowBarSection({ settings, update }: FlowBarSectionProps) {
  const { t } = useTranslation();
  const { flowBar } = settings;
  const [openedAt] = useState(() => Date.now());
  const hidden = flowBar.hiddenUntil * 1000 > openedAt;
  return (
    <section className="card stack" aria-labelledby="flow-bar-heading">
      <h2 id="flow-bar-heading">{t("flowBarSettings.title")}</h2>
      <ToggleRow
        label={t("flowBarSettings.alwaysShow")}
        description={t("flowBarSettings.alwaysShowBody")}
        checked={flowBar.alwaysShow}
        onChange={(alwaysShow) => void update({ flowBar: { alwaysShow } })}
      />
      {hidden && (
        <div className="row">
          <span className="muted">
            {t("flowBarSettings.hiddenUntil", {
              time: new Date(flowBar.hiddenUntil * 1000).toLocaleTimeString([], {
                hour: "numeric",
                minute: "2-digit",
              }),
            })}
          </span>
          <button type="button" onClick={() => void update({ flowBar: { hiddenUntil: 0 } })}>
            {t("flowBarSettings.showNow")}
          </button>
        </div>
      )}
      <p className="muted">{t("flowBarSettings.note")}</p>
    </section>
  );
}
