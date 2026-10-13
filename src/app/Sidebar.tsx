export type PageId = "home" | "dictionary" | "snippets" | "style" | "settings";

const ITEMS: { id: PageId; label: string }[] = [
  { id: "home", label: "Home" },
  { id: "dictionary", label: "Dictionary" },
  { id: "snippets", label: "Snippets" },
  { id: "style", label: "Style" },
  { id: "settings", label: "Settings" },
];

interface SidebarProps {
  page: PageId;
  onSelect: (page: PageId) => void;
}

export function Sidebar({ page, onSelect }: SidebarProps) {
  return (
    <nav className="sidebar" aria-label="Main">
      {ITEMS.map((item) => (
        <button
          key={item.id}
          type="button"
          className="sidebar-item"
          aria-current={item.id === page ? "page" : undefined}
          onClick={() => onSelect(item.id)}
        >
          {item.label}
        </button>
      ))}
    </nav>
  );
}
