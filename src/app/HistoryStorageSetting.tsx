import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { Section } from "@/components/settings";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { errorMessage, setHistoryStorage, type HistoryStorage } from "../lib/ipc";
import { ConfirmDialog } from "./ConfirmDialog";
import { Dialog, DialogFooter } from "./Dialog";
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
    <Section headingId="history-storage-heading" title={t("privacy.storageTitle")}>
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
        <p className="px-1 text-sm text-destructive" role="alert">
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
    </Section>
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
      <p className="text-muted-foreground">{t("privacy.neverBody")}</p>
      <div className="flex flex-col gap-1.5">
        <Label htmlFor={fieldId}>{t("privacy.neverField")}</Label>
        <Input
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
      </div>
      {error && (
        <p className="text-sm text-destructive" role="alert">
          {error}
        </p>
      )}
      <DialogFooter>
        <Button onClick={onClose}>{t("common.cancel")}</Button>
        <Button variant="destructive" disabled={!confirmed || busy} onClick={confirm}>
          {t("privacy.neverConfirm")}
        </Button>
      </DialogFooter>
    </Dialog>
  );
}
