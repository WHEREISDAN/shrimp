import { useState } from "react";
import { useTranslation } from "react-i18next";
import { errorMessage } from "../lib/ipc";
import { Button } from "@/components/ui/button";
import { Dialog, DialogFooter } from "./Dialog";

interface ConfirmDialogProps {
  title: string;
  message?: string | undefined;
  confirmLabel: string;
  onConfirm: () => Promise<void>;
  onCancel: () => void;
}

export function ConfirmDialog({
  title,
  message,
  confirmLabel,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  const { t } = useTranslation();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const confirm = () => {
    setBusy(true);
    setError(null);
    onConfirm().catch((err: unknown) => {
      setError(errorMessage(err));
      setBusy(false);
    });
  };

  return (
    <Dialog title={title} onClose={onCancel}>
      {message && <p className="text-muted-foreground">{message}</p>}
      {error && (
        <p className="text-sm text-destructive" role="alert">
          {error}
        </p>
      )}
      <DialogFooter>
        <Button onClick={onCancel} autoFocus>
          {t("common.cancel")}
        </Button>
        <Button variant="destructive" onClick={confirm} disabled={busy}>
          {confirmLabel}
        </Button>
      </DialogFooter>
    </Dialog>
  );
}
