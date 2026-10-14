import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Group, Row, Section } from "@/components/settings";
import { Button } from "@/components/ui/button";
import { deleteAllHistory } from "../lib/ipc";
import { ConfirmDialog } from "./ConfirmDialog";

interface DeleteTranscriptsSettingProps {
  /** "Never store data locally" is on, so there is nothing to delete. */
  disabled: boolean;
}

/** Data and privacy › Delete all transcripts (HIS-09), apart from Local data storage. */
export function DeleteTranscriptsSetting({ disabled }: DeleteTranscriptsSettingProps) {
  const { t } = useTranslation();
  const [asking, setAsking] = useState(false);
  const [done, setDone] = useState(false);

  return (
    <Section headingId="transcripts-heading" title={t("privacy.transcriptsTitle")}>
      <Group>
        <Row>
          <div className="flex min-w-0 flex-1 basis-56 flex-col gap-0.5">
            <p>{t("privacy.deleteAllBody")}</p>
            <p className="text-sm text-success" role="status">
              {done ? t("privacy.deleteAllDone") : null}
            </p>
          </div>
          <Button
            variant="destructive"
            size="sm"
            disabled={disabled}
            onClick={() => {
              setDone(false);
              setAsking(true);
            }}
          >
            {t("privacy.deleteAll")}
          </Button>
        </Row>
      </Group>
      {asking && (
        <ConfirmDialog
          title={t("privacy.deleteAllTitle")}
          message={t("privacy.deleteAllWarning")}
          confirmLabel={t("privacy.deleteAllConfirm")}
          onCancel={() => setAsking(false)}
          onConfirm={async () => {
            await deleteAllHistory();
            setAsking(false);
            setDone(true);
          }}
        />
      )}
    </Section>
  );
}
