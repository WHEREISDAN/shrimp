import { Trans, useTranslation } from "react-i18next";
import { Group, Row, Section } from "@/components/settings";
import type { Settings, SettingsPatch } from "../lib/ipc";
import { ToggleRow } from "./ToggleRow";

interface VibeCodingSectionProps {
  settings: Settings;
  update: (patch: SettingsPatch) => Promise<void>;
}

export function VibeCodingSection({ settings, update }: VibeCodingSectionProps) {
  const { t } = useTranslation();
  return (
    <Section headingId="vibe-coding-heading" title={t("vibeCoding.title")}>
      <Group>
        <ToggleRow
          label={t("vibeCoding.fileTagging")}
          description={t("vibeCoding.fileTaggingBody")}
          checked={settings.vibeCoding.fileTagging}
          onChange={(fileTagging) => void update({ vibeCoding: { fileTagging } })}
        />
        <Row>
          <div className="flex min-w-0 flex-1 flex-col gap-0.5">
            <p>{t("vibeCoding.variables")}</p>
            <p className="text-sm text-muted-foreground">
              <Trans
                i18nKey="vibeCoding.variablesBody"
                components={{
                  code: (
                    <code className="rounded-xs bg-muted px-1 py-px font-mono text-[0.92em] text-foreground" />
                  ),
                }}
              />
            </p>
            <ol className="mt-1.5 flex list-decimal flex-col gap-0.5 pl-4 text-sm text-muted-foreground marker:tabular-nums">
              <li>{t("vibeCoding.step1")}</li>
              <li>{t("vibeCoding.step2")}</li>
              <li>{t("vibeCoding.step3")}</li>
            </ol>
          </div>
        </Row>
      </Group>
    </Section>
  );
}
