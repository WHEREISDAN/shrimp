import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Group, Note, Row, Section } from "@/components/settings";
import { Button } from "@/components/ui/button";
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
    <Section headingId="flow-bar-heading" title={t("flowBarSettings.title")}>
      <Group>
        <ToggleRow
          label={t("flowBarSettings.alwaysShow")}
          description={t("flowBarSettings.alwaysShowBody")}
          checked={flowBar.alwaysShow}
          onChange={(alwaysShow) => void update({ flowBar: { alwaysShow } })}
        />
        {hidden && (
          <Row>
            <span className="min-w-0 flex-1 text-muted-foreground tabular-nums">
              {t("flowBarSettings.hiddenUntil", {
                time: new Date(flowBar.hiddenUntil * 1000).toLocaleTimeString([], {
                  hour: "numeric",
                  minute: "2-digit",
                }),
              })}
            </span>
            <Button size="sm" onClick={() => void update({ flowBar: { hiddenUntil: 0 } })}>
              {t("flowBarSettings.showNow")}
            </Button>
          </Row>
        )}
      </Group>
      <Note>{t("flowBarSettings.note")}</Note>
    </Section>
  );
}
