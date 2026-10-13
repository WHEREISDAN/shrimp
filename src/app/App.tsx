import { useState } from "react";
import { EVENTS } from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";
import { DictionaryPage } from "./DictionaryPage";
import { HomePage } from "./HomePage";
import { InsightsPage } from "./InsightsPage";
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
  const [revealMicrophone, setRevealMicrophone] = useState(0);
  const [revealLanguages, setRevealLanguages] = useState(0);
  const [revealVoiceProfile, setRevealVoiceProfile] = useState(0);
  const openShortcuts = () => setShortcutsOpen(true);

  useTauriEvent<string>(EVENTS.navigate, (section) => {
    if (section === "shortcuts") {
      setShortcutsOpen(true);
    } else if (section === "settings") {
      setPage("settings");
    } else if (section === "history") {
      // Home lists recent dictations (HIS-01).
      setPage("home");
    } else if (section === "dictionary") {
      setPage("dictionary");
    } else if (section === "insights") {
      setPage("insights");
    } else if (section === "voiceProfile") {
      setPage("insights");
      setRevealVoiceProfile((count) => count + 1);
    } else if (section === "languages") {
      setPage("settings");
      setRevealLanguages((count) => count + 1);
    } else if (section === "microphone") {
      setPage("settings");
      setRevealMicrophone((count) => count + 1);
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
          revealMicrophone={revealMicrophone}
          revealLanguages={revealLanguages}
          revealVoiceProfile={revealVoiceProfile}
          onNavigate={setPage}
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
  revealMicrophone: number;
  revealLanguages: number;
  revealVoiceProfile: number;
  onNavigate: (page: PageId) => void;
}

function CurrentPage({
  page,
  onOpenShortcuts,
  revealExperimental,
  revealMicrophone,
  revealLanguages,
  revealVoiceProfile,
  onNavigate,
}: CurrentPageProps) {
  switch (page) {
    case "home":
      return <HomePage onOpenInsights={() => onNavigate("insights")} />;
    case "insights":
      return <InsightsPage revealVoiceProfile={revealVoiceProfile} />;
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
        <SettingsPage
          revealExperimental={revealExperimental}
          revealMicrophone={revealMicrophone}
          revealLanguages={revealLanguages}
          onOpenShortcuts={onOpenShortcuts}
        />
      );
  }
}
