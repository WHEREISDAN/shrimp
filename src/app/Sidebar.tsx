import { useTranslation } from "react-i18next";

export type PageId =
  "home" | "insights" | "dictionary" | "snippets" | "style" | "transforms" | "settings";

export const PAGES: PageId[] = [
  "home",
  "insights",
  "dictionary",
  "snippets",
  "style",
  "transforms",
  "settings",
];

interface SidebarProps {
  page: PageId;
  onSelect: (page: PageId) => void;
}

export function Sidebar({ page, onSelect }: SidebarProps) {
  const { t } = useTranslation();
  return (
    <nav className="sidebar" aria-label={t("nav.label")}>
      {PAGES.map((id) => (
        <button
          key={id}
          type="button"
          className="sidebar-item"
          aria-current={id === page ? "page" : undefined}
          onClick={() => onSelect(id)}
        >
          {t(`nav.${id}`)}
        </button>
      ))}
    </nav>
  );
}
