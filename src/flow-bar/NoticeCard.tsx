import { runNoticeAction, type Notice, type NoticeAction } from "../lib/ipc";

const ACTION_LABELS: Record<NoticeAction, string> = {
  copyLastTranscript: "Copy",
  openAccessibilitySettings: "Open Settings",
  openMicrophoneSettings: "Open Settings",
  openKeyboardSettings: "Open Settings",
  retryModelDownload: "Retry",
};

interface NoticeCardProps {
  notice: Notice;
  onDismiss: () => void;
}

export function NoticeCard({ notice, onDismiss }: NoticeCardProps) {
  const { action } = notice;
  return (
    <div
      className={`notice notice-${notice.level}`}
      role={notice.level === "error" ? "alert" : "status"}
    >
      <p className="notice-message">{notice.message}</p>
      {action && (
        <button
          type="button"
          className="notice-action"
          onClick={() => {
            void runNoticeAction(action).catch(() => undefined);
            onDismiss();
          }}
        >
          {ACTION_LABELS[action]}
        </button>
      )}
      <button type="button" className="notice-close" aria-label="Dismiss" onClick={onDismiss}>
        ×
      </button>
    </div>
  );
}
