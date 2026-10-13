import type { ReactNode } from "react";

interface PageProps {
  title: string;
  actions?: ReactNode;
  /** An open dialog; the page behind it becomes inert. */
  modal?: ReactNode;
  children: ReactNode;
}

export function Page({ title, actions, modal, children }: PageProps) {
  const modalOpen = Boolean(modal);
  return (
    <div className="app">
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
