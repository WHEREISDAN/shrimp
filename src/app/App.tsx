import { useState, type ReactNode } from "react";
import { ResizeEdges } from "@/components/resize-edges";
import { WindowControls } from "@/components/window-controls";
import { useInterfaceLanguage } from "../i18n";
import { EVENTS } from "../lib/ipc";
import { IS_WINDOWS } from "../lib/platform";
import { useTauriEvent } from "../lib/useTauriEvent";
import { DictionaryPage } from "./DictionaryPage";
import { ErrorScreen, hasProblem, problemKind } from "./ErrorScreen";
import { Onboarding } from "./onboarding/Onboarding";
import { ReverifyScreen } from "./onboarding/ReverifyScreen";
import { usePermissions } from "./onboarding/usePermissions";
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
  const screen = <Screen />;
  return IS_WINDOWS ? <WindowChrome>{screen}</WindowChrome> : screen;
}

/**
 * Windows draws no title bar for the main window, so every screen gets its
 * caption buttons and resize borders.
 */
function WindowChrome({ children }: { children: ReactNode }) {
  return (
    <>
      {children}
      <WindowControls />
      <ResizeEdges />
    </>
  );
}

function Screen() {
  const { page, go } = usePageHistory();
  const [scrolled, setScrolled] = useState(false);
  const { settings, error: settingsError, update } = useSettings();
  const health = useAppHealth();
  const permissions = usePermissions();
  const [reverifyLater, setReverifyLater] = useState(false);
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
    } else if (
      section === "dictionary" ||
      section === "insights" ||
      section === "help" ||
      section === "transforms"
    ) {
      go(section);
    } else if (section === "voiceProfile") {
      go("insights");
      setRevealVoiceProfile((count) => count + 1);
    }
  });

  if (hasProblem(health)) {
    return <ErrorScreen key={problemKind(health)} health={health} />;
  }
  if (!settings && !settingsError) {
    return null;
  }
  if (settings && !settings.onboarding.completed) {
    return <Onboarding settings={settings} update={update} />;
  }
  // A permission granted before is missing now, usually after an update (ONB-08).
  const granted = settings?.onboarding.granted;
  const lost =
    granted !== undefined &&
    permissions !== null &&
    ((granted.accessibility && permissions.accessibility !== "granted") ||
      (granted.microphone && permissions.microphone !== "granted"));
  if (lost && !reverifyLater) {
    return <ReverifyScreen info={permissions} onLater={() => setReverifyLater(true)} />;
  }
  return (
    <div className="shell flex h-full">
      <Sidebar page={page} onSelect={go} />
      <main
        className="content group/content relative min-w-0 flex-1 overflow-y-auto bg-background"
        data-scrolled={scrolled}
        inert={shortcutsOpen}
        onScroll={(event) => setScrolled(event.currentTarget.scrollTop > 2)}
      >
        <CurrentPage
          page={page}
          settingsTarget={settingsTarget}
          revealVoiceProfile={revealVoiceProfile}
          onNavigate={go}
          onOpenShortcuts={() => setShortcutsOpen(true)}
          onSetup={() => void update({ onboarding: { completed: false, step: "welcome" } })}
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
  onSetup: () => void;
}

function CurrentPage({
  page,
  settingsTarget,
  revealVoiceProfile,
  onNavigate,
  onOpenShortcuts,
  onSetup,
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
      return <HelpPage onOpenShortcuts={onOpenShortcuts} onSetup={onSetup} />;
  }
}
