import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { errorMessage, setHistoryStorage, type HistoryStorage } from "../lib/ipc";
import { ConfirmDialog } from "./ConfirmDialog";
import { Dialog } from "./Dialog";
import { RadioCards } from "./RadioCards";

/** What "Never store data locally" asks the user to type. */
const DELETE_ALL = "delete all";

interface HistoryStorageSettingProps {
  value: HistoryStorage;
}

/** Data and privacy › Local data storage (HIS-07). */
export function HistoryStorageSetting({ value }: HistoryStorageSettingProps) {
  const { t } = useTranslation();
  const [asking, setAsking] = useState<HistoryStorage | null>(null);
  const [error, setError] = useState<string | null>(null);

  const choose = (storage: HistoryStorage) => {
    if (storage === value) {
      return;
    }
    if (storage === "local") {
      setHistoryStorage("local").catch((err: unknown) => setError(errorMessage(err)));
    } else {
      setAsking(storage);
    }
  };

  return (
    <div className="stack-tight">
      <p className="card-title">{t("privacy.storageTitle")}</p>
      <RadioCards
        label={t("privacy.storageTitle")}
        value={value}
        onChange={choose}
        options={[
          {
            value: "local",
            title: t("privacy.storageLocal"),
            description: t("privacy.storageLocalBody"),
          },
          {
            value: "autoDelete",
            title: t("privacy.storageAutoDelete"),
            description: t("privacy.storageAutoDeleteBody"),
          },
          {
            value: "never",
            title: t("privacy.storageNever"),
            description: t("privacy.storageNeverBody"),
          },
        ]}
      />
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {asking === "autoDelete" && (
        <ConfirmDialog
          title={t("privacy.autoDeleteTitle")}
          message={t("privacy.storageAutoDeleteBody")}
          confirmLabel={t("privacy.autoDeleteConfirm")}
          onCancel={() => setAsking(null)}
          onConfirm={async () => {
            await setHistoryStorage("autoDelete");
            setAsking(null);
          }}
        />
      )}
      {asking === "never" && <DeleteAllDialog onClose={() => setAsking(null)} />}
    </div>
  );
}

/** "Never store data locally": confirmed by typing "delete all". */
function DeleteAllDialog({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation();
  const fieldId = useId();
  const [typed, setTyped] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const confirmed = typed.trim().toLowerCase() === DELETE_ALL;

  const confirm = () => {
    setBusy(true);
    setHistoryStorage("never", typed)
      .then(onClose)
      .catch((err: unknown) => {
        setError(errorMessage(err));
        setBusy(false);
      });
  };

  return (
    <Dialog title={t("privacy.neverTitle")} onClose={onClose}>
      <p className="muted">{t("privacy.neverBody")}</p>
      <label htmlFor={fieldId} className="card-title">
        {t("privacy.neverField")}
      </label>
      <input
        id={fieldId}
        type="text"
        autoComplete="off"
        placeholder={DELETE_ALL}
        value={typed}
        onChange={(event) => setTyped(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && confirmed && !busy) {
            confirm();
          }
        }}
      />
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <div className="dialog-actions">
        <button type="button" onClick={onClose}>
          {t("common.cancel")}
        </button>
        <button type="button" className="danger" disabled={!confirmed || busy} onClick={confirm}>
          {t("privacy.neverConfirm")}
        </button>
      </div>
    </Dialog>
  );
}
