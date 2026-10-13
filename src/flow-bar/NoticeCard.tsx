import { runNoticeAction, type Notice, type NoticeAction } from "../lib/ipc";

const ACTION_LABELS: Record<NoticeAction, string> = {
  copyLastTranscript: "Copy",
  openAccessibilitySettings: "Open Settings",
  openMicrophoneSettings: "Open Settings",
  openKeyboardSettings: "Open Settings",
  retryModelDownload: "Retry",
  openAiSettings: "Open Settings",
  openExperimentalSettings: "Go to settings",
  viewTransformChanges: "See changes",
  openDictionary: "View",
  undoCancel: "Undo",
  chooseMicrophone: "Choose microphone",
  switchMicrophone: "Switch microphone",
  switchToNewMicrophone: "Switch",
  keepMicrophone: "Don't switch",
  dismissBluetoothAdvice: "Don't show again",
  openSoundSettings: "Open Sound Settings",
  troubleshootMicrophone: "Troubleshoot",
  openLanguageSettings: "Open language settings",
  addDetectedLanguage: "Add language",
  dismissLanguageCard: "Don't show again",
  showFlowBar: "Undo",
};

interface NoticeCardProps {
  notice: Notice;
  onDismiss: () => void;
}

export function NoticeCard({ notice, onDismiss }: NoticeCardProps) {
  return (
    <div
      className={`notice notice-${notice.level}`}
      role={notice.level === "error" ? "alert" : "status"}
    >
      <p className="notice-message">{notice.message}</p>
      {notice.actions.map((action) => (
        <button
          key={action}
          type="button"
          className="notice-action"
          onClick={() => {
            void runNoticeAction(action).catch(() => undefined);
            onDismiss();
          }}
        >
          {ACTION_LABELS[action]}
        </button>
      ))}
      <button type="button" className="notice-close" aria-label="Dismiss" onClick={onDismiss}>
        ×
      </button>
    </div>
  );
}
