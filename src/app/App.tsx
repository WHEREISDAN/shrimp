import { useState } from "react";
import { EVENTS } from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";
import { DictionaryPage } from "./DictionaryPage";
import { HomePage } from "./HomePage";
import { SettingsPage } from "./SettingsPage";
import { ShortcutsDialog } from "./ShortcutsDialog";
import { Sidebar, type PageId } from "./Sidebar";
import { SnippetsPage } from "./SnippetsPage";
import { StylePage } from "./StylePage";
import { TransformsPage } from "./TransformsPage";

export function App() {
  const [page, setPage] = useState<PageId>("home");
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [revealExperimental, setRevealExperimental] = useState(0);
  const openShortcuts = () => setShortcutsOpen(true);

  useTauriEvent<string>(EVENTS.navigate, (section) => {
    if (section === "shortcuts") {
      setShortcutsOpen(true);
    } else if (section === "settings") {
      setPage("settings");
    } else if (section === "dictionary") {
      setPage("dictionary");
    } else if (section === "experimental") {
      setPage("settings");
      setRevealExperimental((count) => count + 1);
    }
  });

  return (
    <div className="shell">
      <Sidebar page={page} onSelect={setPage} />
      <main className="content" inert={shortcutsOpen}>
        <CurrentPage
          page={page}
          onOpenShortcuts={openShortcuts}
          revealExperimental={revealExperimental}
        />
      </main>
      {shortcutsOpen && <ShortcutsDialog onClose={() => setShortcutsOpen(false)} />}
    </div>
  );
}

interface CurrentPageProps {
  page: PageId;
  onOpenShortcuts: () => void;
  revealExperimental: number;
}

function CurrentPage({ page, onOpenShortcuts, revealExperimental }: CurrentPageProps) {
  switch (page) {
    case "home":
      return <HomePage onOpenShortcuts={onOpenShortcuts} />;
    case "dictionary":
      return <DictionaryPage />;
    case "snippets":
      return <SnippetsPage />;
    case "style":
      return <StylePage />;
    case "transforms":
      return <TransformsPage />;
    case "settings":
      return (
        <SettingsPage revealExperimental={revealExperimental} onOpenShortcuts={onOpenShortcuts} />
      );
  }
}
