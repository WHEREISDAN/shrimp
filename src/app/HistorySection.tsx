import { Search } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import {
  copyHistory,
  deleteHistory,
  errorMessage,
  extractHistoryAudio,
  historyAudio,
  retryHistory,
  setHistoryRaw,
  type HistoryItem,
} from "../lib/ipc";
import { ConfirmDialog } from "./ConfirmDialog";
import { groupByDay } from "./historyGroups";
import { HistoryRow, type HistoryActions } from "./HistoryRow";
import { useHistory } from "./useHistory";
import { primaryKey } from "../lib/platform";

/** How long a status message stays. */
const STATUS_MS = 3000;

interface Status {
  text: string;
  error: boolean;
}

const isTyping = (target: EventTarget | null) =>
  target instanceof HTMLElement &&
  (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));

/** Home's transcript history (HIS-01..03): grouped by day, searchable, keyboard driven. */
export function HistorySection() {
  const { t, i18n } = useTranslation();
  const [query, setQuery] = useState("");
  const { list, error } = useHistory(query);
  const [currentId, setCurrentId] = useState<number | null>(null);
  const [status, setStatus] = useState<Status | null>(null);
  const [playing, setPlaying] = useState<{ id: number; url: string } | null>(null);
  const [retrying, setRetrying] = useState<number | null>(null);
  const [deleting, setDeleting] = useState<HistoryItem | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const entries = useMemo(() => list?.entries ?? [], [list]);
  const groups = useMemo(() => groupByDay(entries, t, i18n.language), [entries, t, i18n.language]);
  const current = entries.some((entry) => entry.id === currentId) ? currentId : entries[0]?.id;

  useEffect(() => {
    if (!status) {
      return;
    }
    const timer = setTimeout(() => setStatus(null), STATUS_MS);
    return () => clearTimeout(timer);
  }, [status]);

  useEffect(
    () => () => {
      if (playing) {
        URL.revokeObjectURL(playing.url);
      }
    },
    [playing],
  );

  // Cmd+F (Ctrl+F on Windows) or / searches when not typing (HIS-02).
  useEffect(() => {
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      const find = event.key === "f" && primaryKey(event) && !event.shiftKey && !event.altKey;
      if (find || (event.key === "/" && !isTyping(event.target))) {
        event.preventDefault();
        searchRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const report = (text: string, isError = false) => setStatus({ text, error: isError });
  const fail = (err: unknown) => report(errorMessage(err), true);

  const actions: HistoryActions = {
    activate: (item) => {
      if (item.retry) {
        actions.retry(item);
      } else if (["inserted", "copied", "transcribed"].includes(item.status)) {
        copyHistory(item.id)
          .then(() => report(t("history.copied")))
          .catch(fail);
      }
    },
    retry: (item) => {
      setRetrying(item.id);
      report(t("history.retrying"));
      retryHistory(item.id).catch((err: unknown) => {
        setRetrying(null);
        fail(err);
      });
    },
    toggleRaw: (item) => {
      setHistoryRaw(item.id, !item.showRaw)
        .then(() => item.showRaw || report(t("history.copied")))
        .catch(fail);
    },
    play: (item) => {
      if (playing?.id === item.id) {
        setPlaying(null);
        return;
      }
      historyAudio(item.id)
        .then((bytes) => {
          const url = URL.createObjectURL(new Blob([bytes], { type: "audio/wav" }));
          setPlaying({ id: item.id, url });
        })
        .catch(fail);
    },
    extract: (item) => {
      extractHistoryAudio(item.id)
        .then((path) => report(t("history.extracted", { path })))
        .catch(fail);
    },
    remove: setDeleting,
  };

  // ↑/↓ or k/j move between rows; Esc leaves the list (HIS-02).
  const onListKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const target = event.target as HTMLElement;
    if (target.closest("[role=menu]") || target.tagName === "AUDIO") {
      return;
    }
    const step = { ArrowDown: 1, j: 1, ArrowUp: -1, k: -1 }[event.key];
    if (event.key === "Escape") {
      event.preventDefault();
      searchRef.current?.focus();
      return;
    }
    if (!step || event.metaKey || event.ctrlKey || event.altKey) {
      return;
    }
    event.preventDefault();
    const index = entries.findIndex((entry) => entry.id === current);
    const next = entries[Math.min(Math.max(index + step, 0), entries.length - 1)];
    if (next) {
      setCurrentId(next.id);
      listRef.current?.querySelector<HTMLElement>(`[data-history-id="${next.id}"]`)?.focus();
    }
  };

  const empty = list && entries.length === 0;
  return (
    <section className="history flex flex-col gap-3" aria-label={t("history.region")}>
      <div className="history-head flex flex-wrap items-center justify-between gap-x-4 gap-y-2 px-1">
        <h2 className="section-title text-base font-semibold">{t("history.title")}</h2>
        <div className="relative w-60 max-w-full">
          <Search
            aria-hidden="true"
            className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground"
          />
          <Input
            ref={searchRef}
            type="search"
            className="history-search pl-8"
            aria-label={t("history.search")}
            placeholder={t("history.searchPlaceholder")}
            value={query}
            disabled={list?.disabled}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "ArrowDown" && current !== undefined) {
                event.preventDefault();
                listRef.current
                  ?.querySelector<HTMLElement>(`[data-history-id="${current}"]`)
                  ?.focus();
              }
            }}
          />
        </div>
      </div>
      <p
        className={cn(
          "px-1 text-sm empty:hidden",
          status?.error ? "text-destructive" : "text-muted-foreground",
        )}
        role="status"
        aria-live="polite"
      >
        {status?.text}
      </p>
      {error && (
        <p className="px-1 text-sm text-destructive" role="alert">
          {error}
        </p>
      )}
      {!list && !error && <p className="px-1 text-muted-foreground">{t("common.loading")}</p>}
      {list?.disabled && (
        <div className="empty flex flex-col items-center gap-1 rounded-xl border border-dashed px-6 py-10 text-center">
          <p className="font-medium">{t("history.disabled")}</p>
          <p className="max-w-sm text-sm text-muted-foreground">{t("history.disabledHint")}</p>
        </div>
      )}
      {empty && !list.disabled && (
        <p className="empty rounded-xl border border-dashed px-6 py-10 text-center text-muted-foreground">
          {query ? t("history.noMatches", { query }) : t("history.empty")}
        </p>
      )}
      <div ref={listRef} onKeyDown={onListKeyDown} className="flex flex-col gap-5">
        {groups.map((group) => (
          <section
            key={group.key}
            className="history-day flex flex-col gap-1.5"
            aria-labelledby={`day-${group.key}`}
          >
            <h3
              id={`day-${group.key}`}
              className="history-day-title px-1 text-sm font-medium text-muted-foreground"
            >
              {group.label}
            </h3>
            <ul
              className="list flex flex-col divide-y divide-separator rounded-xl border bg-card shadow-xs"
              aria-label={group.label}
            >
              {group.entries.map((item: HistoryItem) => (
                <HistoryRow
                  key={item.id}
                  item={item}
                  current={item.id === current}
                  audioUrl={playing?.id === item.id ? playing.url : null}
                  busy={retrying === item.id && item.retry !== null}
                  actions={actions}
                  onFocus={() => setCurrentId(item.id)}
                />
              ))}
            </ul>
          </section>
        ))}
      </div>
      {deleting && (
        <ConfirmDialog
          title={t("history.deleteTitle")}
          message={t("history.deleteWarning")}
          confirmLabel={t("history.deleteConfirm")}
          onCancel={() => setDeleting(null)}
          onConfirm={async () => {
            await deleteHistory(deleting.id);
            setDeleting(null);
            report(t("history.deleted"));
          }}
        />
      )}
    </section>
  );
}
