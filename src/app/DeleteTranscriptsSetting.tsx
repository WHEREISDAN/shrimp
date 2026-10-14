import { useState } from "react";
import { useTranslation } from "react-i18next";
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
    <>
      <div className="toggle-row">
        <div className="toggle-text">
          <p className="card-title">{t("privacy.transcriptsTitle")}</p>
          <p className="muted">{t("privacy.deleteAllBody")}</p>
          <p className="muted" role="status">
            {done ? t("privacy.deleteAllDone") : null}
          </p>
        </div>
        <button
          type="button"
          className="danger"
          disabled={disabled}
          onClick={() => {
            setDone(false);
            setAsking(true);
          }}
        >
          {t("privacy.deleteAll")}
        </button>
      </div>
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
    </>
  );
}
