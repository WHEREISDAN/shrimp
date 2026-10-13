import { useTranslation } from "react-i18next";
import type { HistoryItem } from "../lib/ipc";
import { languageName } from "./historyGroups";
import { MenuButton, type MenuItem } from "./MenuButton";

export interface HistoryActions {
  /** Click or Enter: copy a transcript, or retry or recover a failed one. */
  activate: (item: HistoryItem) => void;
  retry: (item: HistoryItem) => void;
  toggleRaw: (item: HistoryItem) => void;
  play: (item: HistoryItem) => void;
  extract: (item: HistoryItem) => void;
  remove: (item: HistoryItem) => void;
}

interface HistoryRowProps {
  item: HistoryItem;
  /** The row that takes Tab focus in the list (roving focus). */
  current: boolean;
  /** The recording's blob URL while it plays. */
  audioUrl: string | null;
  busy: boolean;
  actions: HistoryActions;
  onFocus: () => void;
}

const hasText = (item: HistoryItem) =>
  item.status === "inserted" || item.status === "copied" || item.status === "transcribed";

export function HistoryRow({ item, current, audioUrl, busy, actions, onFocus }: HistoryRowProps) {
  const { t, i18n } = useTranslation();
  const tabIndex = current ? 0 : -1;
  const time = new Intl.DateTimeFormat(i18n.language, { timeStyle: "short" }).format(
    new Date(item.createdAt),
  );
  const text = hasText(item)
    ? item.showRaw
      ? item.rawText
      : item.finalText
    : item.status === "interrupted"
      ? t("history.statusInterrupted")
      : item.status === "empty"
        ? t("history.statusEmpty")
        : (item.error ?? t("history.statusFailed"));
  const edited = hasText(item) && item.rawText.trim() !== item.finalText.trim();
  const language = languageName(item.language, i18n.language);

  const menu: MenuItem[] = [];
  if (item.retry === "retry") {
    menu.push({ label: t("history.retryTranscript"), onSelect: () => actions.retry(item) });
  }
  if (edited) {
    menu.push({
      label: item.showRaw ? t("history.redoAiEdit") : t("history.undoAiEdit"),
      onSelect: () => actions.toggleRaw(item),
    });
  }
  if (item.hasAudio) {
    menu.push(
      {
        label: audioUrl ? t("history.stopAudio") : t("history.playAudio"),
        onSelect: () => actions.play(item),
      },
      { label: t("history.extractAudio"), onSelect: () => actions.extract(item) },
    );
  }
  menu.push({
    label: t("history.deleteTranscript"),
    onSelect: () => actions.remove(item),
    danger: true,
  });

  return (
    <li className={hasText(item) ? "history-row" : "history-row history-row-problem"}>
      <button
        type="button"
        className="history-main"
        data-history-id={item.id}
        tabIndex={tabIndex}
        onFocus={onFocus}
        onClick={() => actions.activate(item)}
      >
        <span className="history-time">{time}</span>
        <span className="history-text">{text}</span>
        <span className="history-meta">
          <span>{item.appName ?? t("history.unknownApp")}</span>
          {language && <span>{language}</span>}
          {item.status === "copied" && <span className="badge">{t("history.notPasted")}</span>}
          {hasText(item) && !item.formatted && (
            <span className="badge" title={t("history.unformattedHint")}>
              {t("history.unformatted")}
            </span>
          )}
          {item.showRaw && (
            <span className="badge" title={t("history.rawShownHint")}>
              {t("history.rawShown")}
            </span>
          )}
        </span>
      </button>
      <div className="history-actions">
        {item.retry && (
          <button
            type="button"
            tabIndex={tabIndex}
            disabled={busy}
            onClick={() => actions.retry(item)}
          >
            {item.retry === "recover" ? t("history.recover") : t("history.retry")}
          </button>
        )}
        <MenuButton label={t("history.moreOptions")} items={menu} tabIndex={tabIndex} />
      </div>
      {audioUrl && (
        <audio
          className="history-audio"
          controls
          autoPlay
          src={audioUrl}
          aria-label={t("history.recording", { seconds: Math.round(item.audioMs / 1000) })}
        />
      )}
    </li>
  );
}
