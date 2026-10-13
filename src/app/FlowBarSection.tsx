import { useState } from "react";
import type { Settings, SettingsPatch } from "../lib/ipc";
import { ToggleRow } from "./ToggleRow";

interface FlowBarSectionProps {
  settings: Settings;
  update: (patch: SettingsPatch) => Promise<void>;
}

/** Settings › Flow Bar (BAR-04, BAR-05). */
export function FlowBarSection({ settings, update }: FlowBarSectionProps) {
  const { flowBar } = settings;
  const [openedAt] = useState(() => Date.now());
  const hidden = flowBar.hiddenUntil * 1000 > openedAt;
  return (
    <section className="card stack" aria-labelledby="flow-bar-heading">
      <h2 id="flow-bar-heading">Flow Bar</h2>
      <ToggleRow
        label="Show Flow Bar at all times"
        description="When off, the bar appears only while you dictate or when Shrimp has something to tell you."
        checked={flowBar.alwaysShow}
        onChange={(alwaysShow) => void update({ flowBar: { alwaysShow } })}
      />
      {hidden && (
        <div className="row">
          <span className="muted">
            Hidden until{" "}
            {new Date(flowBar.hiddenUntil * 1000).toLocaleTimeString([], {
              hour: "numeric",
              minute: "2-digit",
            })}
          </span>
          <button type="button" onClick={() => void update({ flowBar: { hiddenUntil: 0 } })}>
            Show now
          </button>
        </div>
      )}
      <p className="muted">
        Drag the bar to the bottom, left or right edge of the screen; it follows you to the display
        your pointer is on. Right-click it for the microphone and more.
      </p>
    </section>
  );
}
