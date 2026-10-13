import type { ReactNode } from "react";

interface PageProps {
  title: string;
  /** Room for a second column, such as Settings' list of pages. */
  wide?: boolean;
  actions?: ReactNode;
  /** An open dialog; the page behind it becomes inert. */
  modal?: ReactNode;
  children: ReactNode;
}

export function Page({ title, wide = false, actions, modal, children }: PageProps) {
  const modalOpen = Boolean(modal);
  return (
    <div className={wide ? "app app-wide" : "app"}>
      <div className="page" aria-hidden={modalOpen || undefined} inert={modalOpen}>
        <header className="page-header">
          <h1>{title}</h1>
          {actions && <div className="page-actions">{actions}</div>}
        </header>
        {children}
      </div>
      {modal}
    </div>
  );
}
