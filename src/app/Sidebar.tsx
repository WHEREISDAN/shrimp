import { useTranslation } from "react-i18next";

export type PageId =
  "home" | "insights" | "dictionary" | "snippets" | "style" | "transforms" | "settings" | "help";

/** The sidebar's pages, top to bottom; Settings and Help sit at the bottom (APP-04). */
export const PAGES: PageId[] = [
  "home",
  "insights",
  "dictionary",
  "snippets",
  "style",
  "transforms",
  "settings",
  "help",
];

const BOTTOM: PageId[] = ["settings", "help"];

interface SidebarProps {
  page: PageId;
  onSelect: (page: PageId) => void;
}

export function Sidebar({ page, onSelect }: SidebarProps) {
  const { t } = useTranslation();
  const item = (id: PageId) => (
    <button
      key={id}
      type="button"
      className="sidebar-item"
      aria-current={id === page ? "page" : undefined}
      onClick={() => onSelect(id)}
    >
      {t(`nav.${id}`)}
    </button>
  );
  return (
    <nav className="sidebar" aria-label={t("nav.label")}>
      {PAGES.filter((id) => !BOTTOM.includes(id)).map(item)}
      <div className="sidebar-bottom">{BOTTOM.map(item)}</div>
    </nav>
  );
}
