import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";

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
    <div className="menu-anchor">
      <button
        ref={buttonRef}
        type="button"
        className="icon-button"
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
        <svg viewBox="0 0 16 16" aria-hidden="true" width="14" height="14">
          <circle cx="3" cy="8" r="1.4" />
          <circle cx="8" cy="8" r="1.4" />
          <circle cx="13" cy="8" r="1.4" />
        </svg>
      </button>
      {open && (
        <div
          ref={menuRef}
          id={menuId}
          className="menu"
          role="menu"
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
              className={item.danger ? "menu-item danger-text" : "menu-item"}
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
