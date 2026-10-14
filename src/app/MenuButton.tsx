import { Ellipsis } from "lucide-react";
import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import { buttonVariants } from "@/components/ui/button";
import { cn } from "@/lib/utils";

export interface MenuItem {
  label: string;
  onSelect: () => void;
  danger?: boolean;
}

interface MenuButtonProps {
  label: string;
  items: MenuItem[];
  /** Roving focus in a list: -1 keeps the button out of the Tab order. */
  tabIndex?: number;
}

/** A "⋯" button with a menu: arrows move, Enter chooses, Esc closes. */
export function MenuButton({ label, items, tabIndex = 0 }: MenuButtonProps) {
  const [open, setOpen] = useState(false);
  const menuId = useId();
  const buttonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) {
      return;
    }
    menuRef.current?.querySelector<HTMLElement>("[role=menuitem]")?.focus();
    const onPointerDown = (event: PointerEvent) => {
      if (!menuRef.current?.contains(event.target as Node) && event.target !== buttonRef.current) {
        setOpen(false);
      }
    };
    window.addEventListener("pointerdown", onPointerDown);
    return () => window.removeEventListener("pointerdown", onPointerDown);
  }, [open]);

  const close = () => {
    setOpen(false);
    buttonRef.current?.focus();
  };

  const onMenuKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const entries = Array.from(
      event.currentTarget.querySelectorAll<HTMLElement>("[role=menuitem]"),
    );
    const index = entries.indexOf(document.activeElement as HTMLElement);
    const move = (to: number) => entries[(to + entries.length) % entries.length]?.focus();
    if (["ArrowDown", "ArrowUp", "Home", "End", "Escape", "Tab"].includes(event.key)) {
      event.stopPropagation();
    }
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        move(index + 1);
        break;
      case "ArrowUp":
        event.preventDefault();
        move(index - 1);
        break;
      case "Home":
        event.preventDefault();
        move(0);
        break;
      case "End":
        event.preventDefault();
        move(entries.length - 1);
        break;
      case "Escape":
      case "Tab":
        event.preventDefault();
        close();
        break;
    }
  };

  return (
    <div className="menu-anchor relative">
      <button
        ref={buttonRef}
        type="button"
        className={cn(
          buttonVariants({ variant: "ghost", size: "icon-sm" }),
          "text-muted-foreground hover:text-foreground aria-expanded:text-foreground",
        )}
        aria-label={label}
        title={label}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={open ? menuId : undefined}
        tabIndex={tabIndex}
        onClick={(event) => {
          event.stopPropagation();
          setOpen(!open);
        }}
        onKeyDown={(event) => {
          if (event.key === "ArrowDown" && !open) {
            event.preventDefault();
            event.stopPropagation();
            setOpen(true);
          }
        }}
      >
        <Ellipsis aria-hidden="true" />
      </button>
      {open && (
        <div
          ref={menuRef}
          id={menuId}
          className="menu absolute top-[calc(100%+4px)] right-0 z-30 flex min-w-44 origin-top-right animate-menu-in flex-col rounded-lg border bg-popover/95 p-1 text-popover-foreground shadow-lg backdrop-blur-xl"
          role="menu"
          tabIndex={-1}
          aria-label={label}
          onKeyDown={onMenuKeyDown}
          onClick={(event) => event.stopPropagation()}
        >
          {items.map((item) => (
            <button
              key={item.label}
              type="button"
              role="menuitem"
              tabIndex={-1}
              className={cn(
                "menu-item flex h-7 items-center rounded-md px-2 text-left text-base whitespace-nowrap outline-none hover:bg-accent focus-visible:bg-accent windows:h-8",
                item.danger && "text-destructive",
              )}
              onClick={() => {
                close();
                item.onSelect();
              }}
            >
              {item.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
