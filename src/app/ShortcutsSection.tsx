import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Group, Note, Row, Section } from "@/components/settings";
import { Button } from "@/components/ui/button";
import { Kbd } from "@/components/ui/kbd";
import { EVENTS, getShortcuts, type ShortcutList } from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";

interface ShortcutsSectionProps {
  onOpenShortcuts: () => void;
}

/** Settings › Shortcuts: what push to talk is, and the way into the Shortcuts dialog. */
export function ShortcutsSection({ onOpenShortcuts }: ShortcutsSectionProps) {
  const { t } = useTranslation();
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
    <Section headingId="shortcut-settings-heading" title={t("shortcutsSection.title")}>
      <Group>
        <Row>
          <p className="flex items-center gap-2">
            {t("shortcutsSection.pushToTalk")} {talk && <Kbd>{talk.shortcuts.join(" / ")}</Kbd>}
          </p>
          <Button size="sm" onClick={onOpenShortcuts}>
            {t("shortcutsSection.change")}
          </Button>
        </Row>
      </Group>
      <Note>{t("shortcutsSection.note")}</Note>
    </Section>
  );
}
