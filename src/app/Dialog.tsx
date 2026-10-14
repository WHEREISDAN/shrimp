import {
  useEffect,
  useId,
  useRef,
  useState,
  type ComponentProps,
  type KeyboardEvent,
  type ReactNode,
} from "react";
import { cn } from "@/lib/utils";

const FOCUSABLE =
  "button:not([disabled]), input:not([disabled]), textarea:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex='-1'])";

interface DialogProps {
  title: string;
  onClose: () => void;
  /** Room for wider content such as the Shortcuts list. */
  wide?: boolean;
  children: ReactNode;
}

/** A modal dialog: Esc closes it, Tab stays inside, and focus returns to where it came from. */
export function Dialog({ title, onClose, wide = false, children }: DialogProps) {
  const titleId = useId();
  const dialogRef = useRef<HTMLDivElement>(null);
  const [opener] = useState(() => document.activeElement);
  const onCloseRef = useRef(onClose);
  useEffect(() => {
    onCloseRef.current = onClose;
  });

  useEffect(() => {
    const dialog = dialogRef.current;
    if (dialog && !dialog.contains(document.activeElement)) {
      dialog.querySelector<HTMLElement>(FOCUSABLE)?.focus();
    }
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onCloseRef.current();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      if (opener instanceof HTMLElement) {
        opener.focus();
      }
    };
  }, [opener]);

  const keepFocusInside = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "Tab") {
      return;
    }
    const items = Array.from(event.currentTarget.querySelectorAll<HTMLElement>(FOCUSABLE));
    const first = items[0];
    const last = items[items.length - 1];
    if (!first || !last) {
      return;
    }
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };

  return (
    <div className="dialog-backdrop fixed inset-0 z-50 flex animate-fade-in items-center justify-center bg-overlay p-6">
      <div
        ref={dialogRef}
        className={cn(
          "dialog flex max-h-full w-full animate-pop-in flex-col gap-4 overflow-y-auto rounded-2xl border bg-popover p-6 text-popover-foreground shadow-xl windows:rounded-lg",
          wide ? "max-w-[36rem]" : "max-w-[30rem]",
        )}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        onKeyDown={keepFocusInside}
      >
        <h2 id={titleId} className="text-lg font-semibold tracking-[-0.01em]">
          {title}
        </h2>
        {children}
      </div>
    </div>
  );
}

/** A dialog's buttons, at its bottom right. */
export function DialogFooter({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      className={cn("dialog-actions flex flex-wrap justify-end gap-2 pt-1", className)}
      {...props}
    />
  );
}
