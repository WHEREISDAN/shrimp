import { useState } from "react";
import { EVENTS } from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";
import { DictionaryPage } from "./DictionaryPage";
import { HomePage } from "./HomePage";
import { SettingsPage } from "./SettingsPage";
import { Sidebar, type PageId } from "./Sidebar";
import { SnippetsPage } from "./SnippetsPage";
import { StylePage } from "./StylePage";
import { TransformsPage } from "./TransformsPage";

export function App() {
  const [page, setPage] = useState<PageId>("home");
  const [revealShortcuts, setRevealShortcuts] = useState(0);
  const [revealExperimental, setRevealExperimental] = useState(0);

  useTauriEvent<string>(EVENTS.navigate, (section) => {
    if (section === "shortcuts") {
      setPage("home");
      setRevealShortcuts((count) => count + 1);
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
      <main className="content">
        <CurrentPage
          page={page}
          revealShortcuts={revealShortcuts}
          revealExperimental={revealExperimental}
        />
      </main>
    </div>
  );
}

interface CurrentPageProps {
  page: PageId;
  revealShortcuts: number;
  revealExperimental: number;
}

function CurrentPage({ page, revealShortcuts, revealExperimental }: CurrentPageProps) {
  switch (page) {
    case "home":
      return <HomePage revealShortcuts={revealShortcuts} />;
    case "dictionary":
      return <DictionaryPage />;
    case "snippets":
      return <SnippetsPage />;
    case "style":
      return <StylePage />;
    case "transforms":
      return <TransformsPage />;
    case "settings":
      return <SettingsPage revealExperimental={revealExperimental} />;
  }
}
