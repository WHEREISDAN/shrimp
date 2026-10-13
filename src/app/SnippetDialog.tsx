import {
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type FormEvent,
  type KeyboardEvent,
} from "react";
import {
  addSnippet,
  errorMessage,
  updateSnippet,
  type Snippet,
  type SnippetSaved,
} from "../lib/ipc";
import { Dialog } from "./Dialog";
import { insertLink, wrapSelection, type TextEdit } from "./markdownEdit";

const MAX_TRIGGER = 60;
const MAX_EXPANSION = 4000;
const EXPANSION_HINT =
  "What Shrimp types. **Bold**, *italic*, lists and [links](https://…) keep their formatting where the app accepts it.";

interface SnippetDialogProps {
  /** The snippet being edited; null adds a new one. */
  snippet: Snippet | null;
  onSaved: (saved: SnippetSaved) => void;
  onClose: () => void;
}

export function SnippetDialog({ snippet, onSaved, onClose }: SnippetDialogProps) {
  const [trigger, setTrigger] = useState(snippet?.trigger ?? "");
  const [expansion, setExpansion] = useState(snippet?.expansion ?? "");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const pendingSelection = useRef<TextEdit | null>(null);
  const ids = useId();

  useLayoutEffect(() => {
    const edit = pendingSelection.current;
    if (edit && textareaRef.current) {
      pendingSelection.current = null;
      textareaRef.current.focus();
      textareaRef.current.setSelectionRange(edit.start, edit.end);
    }
  });

  const save = async () => {
    const input = { trigger, expansion };
    setBusy(true);
    setError(null);
    try {
      onSaved(await (snippet ? updateSnippet(snippet.id, input) : addSnippet(input)));
    } catch (err) {
      setError(errorMessage(err));
      setBusy(false);
    }
  };

  const edit = (apply: (value: string, start: number, end: number) => TextEdit) => {
    const textarea = textareaRef.current;
    if (!textarea) {
      return;
    }
    const next = apply(expansion, textarea.selectionStart, textarea.selectionEnd);
    if (next.value.length <= MAX_EXPANSION) {
      pendingSelection.current = next;
      setExpansion(next.value);
    }
  };
  const bold = () => edit((value, start, end) => wrapSelection(value, start, end, "**"));
  const italic = () => edit((value, start, end) => wrapSelection(value, start, end, "*"));
  const link = () => edit(insertLink);
  const shortcuts: Record<string, () => void> = { b: bold, i: italic, k: link };

  const onTextareaKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    const shortcut = shortcuts[event.key.toLowerCase()];
    if (event.metaKey && !event.shiftKey && !event.altKey && shortcut) {
      event.preventDefault();
      shortcut();
    }
  };

  const onFormKeyDown = (event: KeyboardEvent<HTMLFormElement>) => {
    if (event.metaKey && event.key === "Enter") {
      event.preventDefault();
      void save();
    }
  };

  const onSubmit = (event: FormEvent) => {
    event.preventDefault();
    void save();
  };

  return (
    <Dialog title={snippet ? "Edit snippet" : "Add snippet"} onClose={onClose}>
      <form className="form" onSubmit={onSubmit} onKeyDown={onFormKeyDown}>
        <div className="field">
          <label htmlFor={`${ids}-trigger`}>Snippet</label>
          <input
            id={`${ids}-trigger`}
            type="text"
            value={trigger}
            maxLength={MAX_TRIGGER}
            autoComplete="off"
            autoFocus
            aria-describedby={`${ids}-trigger-hint`}
            onChange={(event) => setTrigger(event.target.value)}
          />
          <p id={`${ids}-trigger-hint`} className="muted hint">
            What you say
          </p>
        </div>
        <div className="field">
          <div className="field-head">
            <label htmlFor={`${ids}-expansion`}>Expansion</label>
            <div className="format-bar">
              <button type="button" onClick={bold}>
                Bold
              </button>
              <button type="button" onClick={italic}>
                Italic
              </button>
              <button type="button" onClick={link}>
                Link
              </button>
            </div>
          </div>
          <textarea
            id={`${ids}-expansion`}
            ref={textareaRef}
            value={expansion}
            maxLength={MAX_EXPANSION}
            rows={6}
            aria-describedby={`${ids}-expansion-hint`}
            onChange={(event) => setExpansion(event.target.value)}
            onKeyDown={onTextareaKeyDown}
          />
          <div className="field-foot">
            <p id={`${ids}-expansion-hint`} className="muted hint">
              {EXPANSION_HINT}
            </p>
            <span className="muted counter">{`${expansion.length}/${MAX_EXPANSION}`}</span>
          </div>
        </div>
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        <div className="dialog-actions">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" className="primary" disabled={busy}>
            {snippet ? "Save changes" : "Save"}
          </button>
        </div>
      </form>
    </Dialog>
  );
}
