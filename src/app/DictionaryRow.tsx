import { useTranslation } from "react-i18next";
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
  return (
    <li className="list-row">
      <input
        type="checkbox"
        aria-label={t("dictionary.select", { term: entry.term })}
        checked={selected}
        onChange={(event) => onSelect(event.target.checked)}
      />
      <div className="list-main">
        <p className="list-title">{title}</p>
        <p className="list-meta">
          {entry.pinned && <span className="badge">{t("dictionary.starred")}</span>}
          {entry.source === "imported" && <span className="badge">{t("dictionary.imported")}</span>}
          {entry.source === "learned" && (
            <span className="badge" title={t("dictionary.learnedHint")}>
              {t("dictionary.learned")}
            </span>
          )}
          {entry.uses > 0 && (
            <span className="muted">{t("dictionary.uses", { count: entry.uses })}</span>
          )}
        </p>
      </div>
      <div className="row-actions">
        <button type="button" aria-pressed={entry.pinned} onClick={onPin}>
          {entry.pinned ? t("dictionary.unstar") : t("dictionary.star")}
        </button>
        <button type="button" onClick={onEdit}>
          {t("dictionary.edit")}
        </button>
        <button type="button" onClick={onDelete}>
          {t("common.delete")}
        </button>
      </div>
    </li>
  );
}
