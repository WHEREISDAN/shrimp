import {
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type FormEvent,
  type KeyboardEvent,
} from "react";
import { Bold, Italic, Link } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import {
  addSnippet,
  errorMessage,
  updateSnippet,
  type Snippet,
  type SnippetSaved,
} from "../lib/ipc";
import { Dialog, DialogFooter } from "./Dialog";
import { insertLink, wrapSelection, type TextEdit } from "./markdownEdit";
import { primaryKey } from "../lib/platform";

const MAX_TRIGGER = 60;
const MAX_EXPANSION = 4000;

interface SnippetDialogProps {
  /** The snippet being edited; null adds a new one. */
  snippet: Snippet | null;
  onSaved: (saved: SnippetSaved) => void;
  onClose: () => void;
}

export function SnippetDialog({ snippet, onSaved, onClose }: SnippetDialogProps) {
  const { t } = useTranslation();
  const [trigger, setTrigger] = useState(snippet?.trigger ?? "");
  const [expansion, setExpansion] = useState(snippet?.expansion ?? "");
  const [error, setError] = useState<string | null>(null);
  /** Formatting would push the expansion past its limit (SNP-01). */
  const [tooLong, setTooLong] = useState(false);
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
    setTooLong(next.value.length > MAX_EXPANSION);
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
    if (primaryKey(event) && !event.shiftKey && !event.altKey && shortcut) {
      event.preventDefault();
      shortcut();
    }
  };

  const onFormKeyDown = (event: KeyboardEvent<HTMLFormElement>) => {
    if (primaryKey(event) && event.key === "Enter") {
      event.preventDefault();
      void save();
    }
  };

  const onSubmit = (event: FormEvent) => {
    event.preventDefault();
    void save();
  };

  const formatButton = "text-muted-foreground hover:text-foreground";

  return (
    <Dialog title={snippet ? t("snippets.editSnippet") : t("snippets.add")} onClose={onClose}>
      <form className="flex flex-col gap-4" onSubmit={onSubmit} onKeyDown={onFormKeyDown}>
        <div className="flex flex-col gap-1.5">
          <label htmlFor={`${ids}-trigger`} className="text-sm font-medium">
            {t("snippets.trigger")}
          </label>
          <Input
            id={`${ids}-trigger`}
            type="text"
            value={trigger}
            maxLength={MAX_TRIGGER}
            autoComplete="off"
            autoFocus
            aria-describedby={`${ids}-trigger-hint`}
            onChange={(event) => setTrigger(event.target.value)}
          />
          <p id={`${ids}-trigger-hint`} className="text-sm text-muted-foreground">
            {t("snippets.triggerHint")}
          </p>
        </div>
        <div className="flex flex-col gap-1.5">
          <div className="flex items-end justify-between gap-3">
            <label htmlFor={`${ids}-expansion`} className="text-sm font-medium">
              {t("snippets.expansion")}
            </label>
            <div className="-mb-0.5 flex items-center gap-0.5">
              <Button
                variant="ghost"
                size="icon-sm"
                className={formatButton}
                aria-label={t("snippets.bold")}
                title={t("snippets.bold")}
                onClick={bold}
              >
                <Bold aria-hidden="true" />
              </Button>
              <Button
                variant="ghost"
                size="icon-sm"
                className={formatButton}
                aria-label={t("snippets.italic")}
                title={t("snippets.italic")}
                onClick={italic}
              >
                <Italic aria-hidden="true" />
              </Button>
              <Button
                variant="ghost"
                size="icon-sm"
                className={formatButton}
                aria-label={t("snippets.link")}
                title={t("snippets.link")}
                onClick={link}
              >
                <Link aria-hidden="true" />
              </Button>
            </div>
          </div>
          <Textarea
            id={`${ids}-expansion`}
            ref={textareaRef}
            value={expansion}
            maxLength={MAX_EXPANSION}
            rows={6}
            aria-describedby={`${ids}-expansion-hint`}
            onChange={(event) => {
              setTooLong(false);
              setExpansion(event.target.value);
            }}
            onKeyDown={onTextareaKeyDown}
          />
          <div className="flex items-start justify-between gap-3">
            <p id={`${ids}-expansion-hint`} className="text-sm text-muted-foreground">
              {t("snippets.expansionHint")}
            </p>
            <span className="shrink-0 pt-px text-xs whitespace-nowrap text-muted-foreground tabular-nums">{`${expansion.length}/${MAX_EXPANSION}`}</span>
          </div>
        </div>
        {(error !== null || tooLong) && (
          <p className="text-sm text-destructive" role="alert">
            {error ?? t("snippets.expansionTooLong")}
          </p>
        )}
        <DialogFooter>
          <Button onClick={onClose}>{t("common.cancel")}</Button>
          <Button type="submit" variant="default" disabled={busy}>
            {snippet ? t("snippets.saveChanges") : t("snippets.save")}
          </Button>
        </DialogFooter>
      </form>
    </Dialog>
  );
}
