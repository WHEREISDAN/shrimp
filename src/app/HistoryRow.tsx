import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
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

/** What a row without a transcript says instead. */
function problemText(item: HistoryItem, t: TFunction): string {
  switch (item.status) {
    case "interrupted":
      return t("history.statusInterrupted");
    case "cancelled":
      return t("history.statusCancelled");
    case "empty":
      return t("history.statusEmpty");
    default:
      return item.error ?? t("history.statusFailed");
  }
}

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
    : problemText(item, t);
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
    <li
      className={cn(
        "history-row group/row grid grid-cols-[minmax(0,1fr)_auto] items-start gap-x-1 pr-2 transition-colors duration-100 first:rounded-t-xl last:rounded-b-xl hover:bg-muted/40 dark:hover:bg-white/[0.025]",
      )}
    >
      <button
        type="button"
        className="history-main grid grid-cols-[4.75rem_minmax(0,1fr)] gap-x-3 gap-y-1 rounded-[11px] py-3 pr-1 pl-4 text-left outline-none focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring/70"
        data-history-id={item.id}
        tabIndex={tabIndex}
        onFocus={onFocus}
        onClick={() => actions.activate(item)}
      >
        <span className="history-time row-span-2 pt-px text-sm text-muted-foreground tabular-nums">
          {time}
        </span>
        <span
          className={cn(
            "history-text whitespace-pre-wrap [overflow-wrap:anywhere]",
            !hasText(item) && "text-muted-foreground italic",
          )}
        >
          {text}
        </span>
        <span className="history-meta flex flex-wrap items-center gap-x-1.5 gap-y-1 text-sm text-muted-foreground">
          <span>{item.appName ?? t("history.unknownApp")}</span>
          {language && (
            <>
              <span aria-hidden="true">·</span>
              <span>{language}</span>
            </>
          )}
          {item.status === "copied" && <Badge className="ml-1">{t("history.notPasted")}</Badge>}
          {hasText(item) && !item.formatted && (
            <Badge className="ml-1" title={t("history.unformattedHint")}>
              {t("history.unformatted")}
            </Badge>
          )}
          {item.showRaw && (
            <Badge className="ml-1" variant="outline" title={t("history.rawShownHint")}>
              {t("history.rawShown")}
            </Badge>
          )}
        </span>
      </button>
      <div className="history-actions flex items-center gap-1 pt-2.5 opacity-0 transition-opacity duration-150 group-focus-within/row:opacity-100 group-hover/row:opacity-100 has-[[aria-expanded=true]]:opacity-100 has-[button:not([aria-haspopup])]:opacity-100">
        {item.retry && (
          <Button size="sm" tabIndex={tabIndex} disabled={busy} onClick={() => actions.retry(item)}>
            {item.retry === "recover" ? t("history.recover") : t("history.retry")}
          </Button>
        )}
        <MenuButton label={t("history.moreOptions")} items={menu} tabIndex={tabIndex} />
      </div>
      {audioUrl && (
        <audio
          className="history-audio col-span-2 mx-4 mb-3 h-8 w-[calc(100%-2rem)]"
          controls
          autoPlay
          src={audioUrl}
          aria-label={t("history.recording", { seconds: Math.round(item.audioMs / 1000) })}
        />
      )}
    </li>
  );
}
