import { useState } from "react";
import { useInterfaceLanguage } from "../i18n";
import { EVENTS } from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";
import { DictionaryPage } from "./DictionaryPage";
import { ErrorScreen, hasProblem, problemKind } from "./ErrorScreen";
import { HelpPage } from "./HelpPage";
import { HomePage } from "./HomePage";
import { InsightsPage } from "./InsightsPage";
import { SettingsPage, type SettingsSection, type SettingsTarget } from "./SettingsPage";
import { ShortcutsDialog } from "./ShortcutsDialog";
import { Sidebar, type PageId } from "./Sidebar";
import { SnippetsPage } from "./SnippetsPage";
import { StylePage } from "./StylePage";
import { TransformsPage } from "./TransformsPage";
import { useAppHealth } from "./useAppHealth";
import { usePageHistory } from "./usePageHistory";
import { useSettings } from "./useSettings";

/** Sections other windows can open with `app://navigate`. */
const SETTINGS_TARGETS: Record<string, Omit<SettingsTarget, "count">> = {
  settings: { section: "general" },
  ai: { section: "ai" },
  microphone: { section: "general", focus: "microphone" },
  languages: { section: "general", focus: "languages" },
  experimental: { section: "experimental", focus: "experimental" },
};

export function App() {
  const { page, go } = usePageHistory();
  const { settings } = useSettings();
  const health = useAppHealth();
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [settingsTarget, setSettingsTarget] = useState<SettingsTarget>({
    section: "general",
    count: 0,
  });
  const [revealVoiceProfile, setRevealVoiceProfile] = useState(0);
  useInterfaceLanguage(settings?.app.language);

  const openSettings = (section: SettingsSection, focus?: SettingsTarget["focus"]) => {
    go("settings");
    setSettingsTarget((current) => ({
      section,
      ...(focus ? { focus } : {}),
      count: current.count + 1,
    }));
  };

  useTauriEvent<string>(EVENTS.navigate, (section) => {
    const target = SETTINGS_TARGETS[section];
    if (target) {
      openSettings(target.section, target.focus);
    } else if (section === "shortcuts") {
      setShortcutsOpen(true);
    } else if (section === "history") {
      go("home");
    } else if (section === "dictionary" || section === "insights") {
      go(section);
    } else if (section === "voiceProfile") {
      go("insights");
      setRevealVoiceProfile((count) => count + 1);
    }
  });

  if (hasProblem(health)) {
    return <ErrorScreen key={problemKind(health)} health={health} />;
  }
  return (
    <div className="shell">
      <Sidebar page={page} onSelect={go} />
      <main className="content" inert={shortcutsOpen}>
        <CurrentPage
          page={page}
          settingsTarget={settingsTarget}
          revealVoiceProfile={revealVoiceProfile}
          onNavigate={go}
          onOpenShortcuts={() => setShortcutsOpen(true)}
        />
      </main>
      {shortcutsOpen && <ShortcutsDialog onClose={() => setShortcutsOpen(false)} />}
    </div>
  );
}

interface CurrentPageProps {
  page: PageId;
  settingsTarget: SettingsTarget;
  revealVoiceProfile: number;
  onNavigate: (page: PageId) => void;
  onOpenShortcuts: () => void;
}

function CurrentPage({
  page,
  settingsTarget,
  revealVoiceProfile,
  onNavigate,
  onOpenShortcuts,
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
          key={settingsTarget.count}
          target={settingsTarget}
          onOpenShortcuts={onOpenShortcuts}
        />
      );
    case "help":
      return <HelpPage onOpenShortcuts={onOpenShortcuts} />;
  }
}
