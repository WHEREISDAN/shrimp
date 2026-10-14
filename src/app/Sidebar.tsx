import {
  BookOpenText,
  ChartNoAxesColumn,
  CircleHelp,
  Feather,
  House,
  Settings,
  TextQuote,
  WandSparkles,
  type LucideIcon,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { BrandMark } from "@/components/brand-mark";
import { IS_WINDOWS } from "@/lib/platform";
import { cn } from "@/lib/utils";

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

const ICONS: Record<PageId, LucideIcon> = {
  home: House,
  insights: ChartNoAxesColumn,
  dictionary: BookOpenText,
  snippets: TextQuote,
  style: Feather,
  transforms: WandSparkles,
  settings: Settings,
  help: CircleHelp,
};

interface SidebarProps {
  page: PageId;
  onSelect: (page: PageId) => void;
}

function Brand() {
  return (
    <div className="flex items-center gap-2 px-2 text-sidebar-foreground">
      <BrandMark className="size-[18px] text-brand" />
      <span className="text-base font-semibold tracking-[-0.01em]">Shrimp</span>
    </div>
  );
}

/**
 * The full-height sidebar. On macOS the window's vibrancy shows through it and
 * the traffic lights sit in its top band; on Windows the band is the title bar.
 */
export function Sidebar({ page, onSelect }: SidebarProps) {
  const { t } = useTranslation();
  const item = (id: PageId) => {
    const Icon = ICONS[id];
    return (
      <button
        key={id}
        type="button"
        className="sidebar-item group relative flex h-7 w-full items-center gap-2 rounded-md px-2 text-left text-base text-sidebar-foreground/90 transition-colors duration-100 outline-none hover:bg-sidebar-accent/60 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring/70 aria-[current=page]:bg-sidebar-accent aria-[current=page]:font-medium aria-[current=page]:text-sidebar-foreground windows:h-9 windows:aria-[current=page]:before:absolute windows:aria-[current=page]:before:top-2.5 windows:aria-[current=page]:before:bottom-2.5 windows:aria-[current=page]:before:left-0 windows:aria-[current=page]:before:w-[3px] windows:aria-[current=page]:before:rounded-full windows:aria-[current=page]:before:bg-brand"
        aria-current={id === page ? "page" : undefined}
        onClick={() => onSelect(id)}
      >
        <Icon
          aria-hidden="true"
          strokeWidth={1.75}
          className="size-4 shrink-0 text-muted-foreground transition-colors group-aria-[current=page]:text-brand"
        />
        <span className="truncate">{t(`nav.${id}`)}</span>
      </button>
    );
  };
  return (
    <aside className="flex w-(--sidebar-w) shrink-0 flex-col border-r border-sidebar-border bg-sidebar">
      <div
        data-tauri-drag-region="deep"
        className={cn(
          "flex h-(--toolbar-h) shrink-0 items-center px-2.5",
          IS_WINDOWS && "h-12 pt-0.5",
        )}
      >
        {IS_WINDOWS && <Brand />}
      </div>
      {!IS_WINDOWS && (
        <div data-tauri-drag-region="deep" className="px-2.5 pb-4">
          <Brand />
        </div>
      )}
      <nav
        className="flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto px-2.5 pb-3"
        aria-label={t("nav.label")}
      >
        {PAGES.filter((id) => !BOTTOM.includes(id)).map(item)}
        <div className="sidebar-bottom mt-auto flex flex-col gap-0.5 pt-3">{BOTTOM.map(item)}</div>
      </nav>
    </aside>
  );
}
