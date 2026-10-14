import { X } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { runNoticeAction, type Notice, type NoticeAction } from "../lib/ipc";

/** How long a pressed action shows its ✓ before the notice goes (NOT-02). */
const CONFIRM_MS = 700;

interface NoticeCardProps {
  notice: Notice;
  onDismiss: () => void;
}

export function NoticeCard({ notice, onDismiss }: NoticeCardProps) {
  const { t } = useTranslation();
  const [confirmed, setConfirmed] = useState<NoticeAction | null>(null);

  useEffect(() => {
    if (!confirmed) {
      return;
    }
    const timer = setTimeout(onDismiss, CONFIRM_MS);
    return () => clearTimeout(timer);
  }, [confirmed, onDismiss]);

  return (
    <div
      className={`notice notice-${notice.level}`}
      role={notice.level === "error" ? "alert" : "status"}
    >
      <p className="notice-message">{notice.message}</p>
      {notice.actions.length > 0 && (
        <div className="notice-actions">
          {notice.actions.map((action) => (
            <button
              key={action}
              type="button"
              className={confirmed === action ? "notice-action confirmed" : "notice-action"}
              disabled={confirmed !== null}
              aria-label={confirmed === action ? t("notice.done") : undefined}
              onClick={() => {
                void runNoticeAction(action).catch(() => undefined);
                setConfirmed(action);
              }}
            >
              {confirmed === action ? "✓" : t(`notice.actions.${action}`)}
            </button>
          ))}
        </div>
      )}
      <button
        type="button"
        className="notice-close"
        aria-label={t("notice.dismiss")}
        onClick={onDismiss}
      >
        <X aria-hidden="true" strokeWidth={2} />
      </button>
    </div>
  );
}
