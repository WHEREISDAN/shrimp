import { useCallback, useEffect, useState } from "react";
import { PAGES, type PageId } from "./Sidebar";

interface History {
  pages: PageId[];
  index: number;
}

/**
 * The page shown, with back and forward (⌘[ and ⌘]) and ⌥↑/⌥↓ through the
 * sidebar (APP-04, A11Y-01). A page that handles ⌥↑/⌥↓ itself (Settings)
 * prevents the default.
 */
export function usePageHistory(first: PageId = "home") {
  const [history, setHistory] = useState<History>({ pages: [first], index: 0 });
  const page = history.pages[history.index] ?? first;

  const go = useCallback((next: PageId) => {
    setHistory((current) => {
      if (current.pages[current.index] === next) {
        return current;
      }
      const pages = [...current.pages.slice(0, current.index + 1), next];
      return { pages, index: pages.length - 1 };
    });
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented) {
        return;
      }
      if (event.metaKey && (event.key === "[" || event.key === "]")) {
        event.preventDefault();
        const step = event.key === "[" ? -1 : 1;
        setHistory((current) => {
          const index = current.index + step;
          return index >= 0 && index < current.pages.length ? { ...current, index } : current;
        });
        return;
      }
      if (
        event.altKey &&
        !event.metaKey &&
        (event.key === "ArrowUp" || event.key === "ArrowDown")
      ) {
        event.preventDefault();
        const index = PAGES.indexOf(page) + (event.key === "ArrowUp" ? -1 : 1);
        const next = PAGES[(index + PAGES.length) % PAGES.length];
        if (next) {
          go(next);
        }
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [page, go]);

  return { page, go };
}
