import type { DictionaryEntry } from "../lib/ipc";

interface DictionaryRowProps {
  entry: DictionaryEntry;
  selected: boolean;
  onSelect: (selected: boolean) => void;
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
  const title = entry.misspelling === null ? entry.term : `${entry.misspelling} → ${entry.term}`;
  return (
    <li className="list-row">
      <input
        type="checkbox"
        aria-label={`Select ${entry.term}`}
        checked={selected}
        onChange={(event) => onSelect(event.target.checked)}
      />
      <div className="list-main">
        <p className="list-title">{title}</p>
        <p className="list-meta">
          {entry.pinned && <span className="badge">Pinned</span>}
          {entry.source === "imported" && <span className="badge">Imported from file</span>}
          {entry.source === "learned" && (
            <span className="badge" title="Added because you corrected it after a dictation">
              Learned
            </span>
          )}
          {entry.uses > 0 && (
            <span className="muted">{`Used ${entry.uses} ${entry.uses === 1 ? "time" : "times"}`}</span>
          )}
        </p>
      </div>
      <div className="row-actions">
        <button type="button" aria-pressed={entry.pinned} onClick={onPin}>
          {entry.pinned ? "Unpin" : "Pin"}
        </button>
        <button type="button" onClick={onEdit}>
          Edit
        </button>
        <button type="button" onClick={onDelete}>
          Delete
        </button>
      </div>
    </li>
  );
}
