import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

interface PageProps {
  title: string;
  /** Room for a second column, such as Settings' list of pages. */
  wide?: boolean;
  actions?: ReactNode;
  /** An open dialog; the page behind it becomes inert. */
  modal?: ReactNode;
  children: ReactNode;
}

/** The column every page's content sits in. */
export const pageColumn = (wide = false) =>
  cn("mx-auto w-full px-8 max-[720px]:px-6", wide ? "max-w-[960px]" : "max-w-[720px]");

/**
 * The bar across a page's top: it drags the window, and once the page scrolls
 * it turns into a translucent toolbar with the title, as in Apple's apps.
 */
export function PageToolbar({ title, wide = false }: { title: string; wide?: boolean }) {
  return (
    <div
      data-tauri-drag-region="deep"
      className="sticky top-0 z-20 h-(--toolbar-h) shrink-0 border-b border-transparent transition-[background-color,border-color] duration-200 group-data-[scrolled=true]/content:border-border group-data-[scrolled=true]/content:bg-background/80 group-data-[scrolled=true]/content:backdrop-blur-xl group-data-[scrolled=true]/content:backdrop-saturate-150"
    >
      <div className={cn(pageColumn(wide), "flex h-full items-center windows:pr-[156px]")}>
        <span
          aria-hidden="true"
          className="truncate text-base font-semibold opacity-0 transition-opacity duration-200 group-data-[scrolled=true]/content:opacity-100"
        >
          {title}
        </span>
      </div>
    </div>
  );
}

export function Page({ title, wide = false, actions, modal, children }: PageProps) {
  const modalOpen = Boolean(modal);
  return (
    <div className={wide ? "app app-wide" : "app"}>
      <div className="page" aria-hidden={modalOpen || undefined} inert={modalOpen}>
        <PageToolbar title={title} wide={wide} />
        <div className={cn(pageColumn(wide), "flex flex-col gap-7 pb-14")}>
          <header
            data-tauri-drag-region="deep"
            className="page-header flex flex-wrap items-center justify-between gap-x-4 gap-y-3"
          >
            <h1 className="font-display text-3xl font-bold tracking-[-0.015em]">{title}</h1>
            {actions && (
              <div className="page-actions flex flex-wrap items-center gap-2">{actions}</div>
            )}
          </header>
          {children}
        </div>
      </div>
      {modal}
    </div>
  );
}
