import { useState } from "react";
import { errorMessage } from "../lib/ipc";
import { Dialog } from "./Dialog";

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
      {message && <p className="muted">{message}</p>}
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <div className="dialog-actions">
        <button type="button" onClick={onCancel} autoFocus>
          Cancel
        </button>
        <button type="button" className="danger" onClick={confirm} disabled={busy}>
          {confirmLabel}
        </button>
      </div>
    </Dialog>
  );
}
