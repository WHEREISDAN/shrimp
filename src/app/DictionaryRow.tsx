import { Star } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { DictionaryEntry } from "../lib/ipc";

interface DictionaryRowProps {
  entry: DictionaryEntry;
  selected: boolean;
  onSelect: (selected: boolean) => void;
  /** Stars or unstars it (the stored field is `pinned`). */
  onPin: () => void;
  onEdit: () => void;
  onDelete: () => void;
}

/** Row actions stay out of the way until the row is hovered or holds focus. */
const revealed =
  "opacity-0 transition-opacity duration-150 group-focus-within/row:opacity-100 group-hover/row:opacity-100";

export function DictionaryRow({
  entry,
  selected,
  onSelect,
  onPin,
  onEdit,
  onDelete,
}: DictionaryRowProps) {
  const { t } = useTranslation();
  const title = entry.misspelling === null ? entry.term : `${entry.misspelling} → ${entry.term}`;
  const starLabel = entry.pinned ? t("dictionary.unstar") : t("dictionary.star");
  return (
    <li
      className={cn(
        "list-row group/row flex min-h-(--row-min-h) items-center gap-3 py-2 pr-2 pl-4 transition-colors duration-100 first:rounded-t-xl last:rounded-b-xl hover:bg-muted/40 dark:hover:bg-white/[0.025]",
        selected && "bg-muted/60 hover:bg-muted/60 dark:bg-white/[0.04]",
      )}
    >
      <input
        type="checkbox"
        aria-label={t("dictionary.select", { term: entry.term })}
        checked={selected}
        onChange={(event) => onSelect(event.target.checked)}
      />
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <p className="list-title font-medium [overflow-wrap:anywhere]">{title}</p>
        <p className="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-muted-foreground empty:hidden">
          {entry.pinned && (
            <Badge variant="brand">
              <Star aria-hidden="true" className="fill-current" />
              {t("dictionary.starred")}
            </Badge>
          )}
          {entry.source === "imported" && <Badge>{t("dictionary.imported")}</Badge>}
          {entry.source === "learned" && (
            <Badge title={t("dictionary.learnedHint")}>{t("dictionary.learned")}</Badge>
          )}
          {entry.uses > 0 && (
            <span className="tabular-nums">{t("dictionary.uses", { count: entry.uses })}</span>
          )}
        </p>
      </div>
      <div className={cn("flex shrink-0 items-center gap-0.5", revealed)}>
        <Button
          variant="ghost"
          size="icon-sm"
          className={cn(
            "text-muted-foreground hover:text-foreground",
            entry.pinned && "text-brand hover:text-brand",
          )}
          aria-label={starLabel}
          title={starLabel}
          aria-pressed={entry.pinned}
          onClick={onPin}
        >
          <Star aria-hidden="true" className={cn(entry.pinned && "fill-current")} />
        </Button>
        <Button variant="ghost" size="sm" onClick={onEdit}>
          {t("dictionary.edit")}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          className="text-destructive hover:bg-destructive/10"
          onClick={onDelete}
        >
          {t("common.delete")}
        </Button>
      </div>
    </li>
  );
}
