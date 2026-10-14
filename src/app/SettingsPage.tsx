import {
  AudioLines,
  CodeXml,
  FlaskConical,
  Lock,
  Monitor,
  SlidersHorizontal,
  Sparkles,
  type LucideIcon,
} from "lucide-react";
import { useEffect, useState, type KeyboardEvent, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import {
  errorMessage,
  getAiOptions,
  getApiKeys,
  getShortcuts,
  type AiOptions,
  type ApiKeys,
} from "../lib/ipc";
import { AiProviderSection } from "./AiProviderSection";
import { AppLanguageSection } from "./AppLanguageSection";
import { CommandModeIntro } from "./CommandModeIntro";
import { ExperimentalSection } from "./ExperimentalSection";
import { FlowBarSection } from "./FlowBarSection";
import { LanguagesSection } from "./LanguagesSection";
import { MicrophoneSection } from "./MicrophoneSection";
import { NotificationsSection } from "./NotificationsSection";
import { Page } from "./Page";
import { PrivacySection } from "./PrivacySection";
import { ShortcutsSection } from "./ShortcutsSection";
import { SoundSection } from "./SoundSection";
import { SpeechEngineSection } from "./SpeechEngineSection";
import { SystemSection } from "./SystemSection";
import { useSettings } from "./useSettings";
import { VibeCodingSection } from "./VibeCodingSection";

/** Settings' pages, in order (APP-05). */
export const SETTINGS_SECTIONS = [
  "general",
  "system",
  "privacy",
  "speech",
  "ai",
  "experimental",
  "vibeCoding",
] as const;
export type SettingsSection = (typeof SETTINGS_SECTIONS)[number];

const SECTION_ICONS: Record<SettingsSection, LucideIcon> = {
  general: SlidersHorizontal,
  system: Monitor,
  privacy: Lock,
  speech: AudioLines,
  ai: Sparkles,
  experimental: FlaskConical,
  vibeCoding: CodeXml,
};

/** Where Settings opens and what it brings into view. */
export interface SettingsTarget {
  section: SettingsSection;
  focus?: "microphone" | "languages" | "experimental";
  /** Increments with each request, so the same one can repeat. */
  count: number;
}

interface SettingsPageProps {
  target?: SettingsTarget;
  onOpenShortcuts?: () => void;
}

const STEP: Record<string, number> = { ArrowDown: 1, ArrowUp: -1 };

/** The page `step` away, wrapping around. */
function neighbour(section: SettingsSection, step: number): SettingsSection {
  const count = SETTINGS_SECTIONS.length;
  const index = (SETTINGS_SECTIONS.indexOf(section) + step + count) % count;
  return SETTINGS_SECTIONS[index] ?? section;
}

export function SettingsPage({
  target = { section: "general", count: 0 },
  onOpenShortcuts = () => undefined,
}: SettingsPageProps) {
  const { t, i18n } = useTranslation();
  const { settings, error, update } = useSettings();
  const [section, setSection] = useState<SettingsSection>(target.section);
  const [options, setOptions] = useState<AiOptions | null>(null);
  const [keys, setKeys] = useState<ApiKeys | null>(null);
  const [commandShortcut, setCommandShortcut] = useState("fn ⌃");
  const [loadError, setLoadError] = useState<string | null>(null);
  const [intro, setIntro] = useState(false);

  useEffect(() => {
    let cancelled = false;
    Promise.all([getAiOptions(), getApiKeys()])
      .then(([loadedOptions, loadedKeys]) => {
        if (!cancelled) {
          setOptions(loadedOptions);
          setKeys(loadedKeys);
        }
      })
      .catch((err: unknown) => !cancelled && setLoadError(errorMessage(err)));
    getShortcuts()
      .then((list) => {
        const command = list.rows.find((row) => row.action === "commandMode");
        const first = command?.shortcuts[0];
        if (!cancelled && first) {
          setCommandShortcut(first);
        }
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);

  // Provider notes come from Rust in the interface language.
  useEffect(() => {
    const reload = () => {
      getAiOptions()
        .then(setOptions)
        .catch(() => undefined);
    };
    i18n.on("languageChanged", reload);
    return () => i18n.off("languageChanged", reload);
  }, [i18n]);

  const move = (step: number) => {
    const next = neighbour(section, step);
    setSection(next);
    document.getElementById(`settings-tab-${next}`)?.focus();
  };

  // Option+↑/↓ moves between settings pages from anywhere on the page (APP-04). It listens
  // in the capture phase so it runs before the sidebar's Option+↑/↓, which then skips it.
  useEffect(() => {
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      const step = STEP[event.key];
      if (step && event.altKey && !event.metaKey && !event.ctrlKey) {
        event.preventDefault();
        setSection(neighbour(section, step));
      }
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [section]);

  // ↑/↓ within the list of pages, as with any tab list.
  const onTabKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const step = STEP[event.key];
    if (!step || event.altKey) {
      return;
    }
    event.preventDefault();
    move(step);
  };

  const closeIntro = (keepOn: boolean) => {
    setIntro(false);
    void update({
      experimental: keepOn
        ? { commandModeIntroSeen: true }
        : { commandModeIntroSeen: true, commandMode: false },
    });
  };

  const reveal = (focus: SettingsTarget["focus"]) =>
    target.focus === focus && target.section === section ? target.count : 0;

  let content: ReactNode = null;
  if (settings && options && keys) {
    switch (section) {
      case "general":
        content = (
          <>
            <ShortcutsSection onOpenShortcuts={onOpenShortcuts} />
            <MicrophoneSection reveal={reveal("microphone")} />
            <LanguagesSection reveal={reveal("languages")} />
            <AppLanguageSection settings={settings} update={update} />
          </>
        );
        break;
      case "system":
        content = (
          <>
            <SystemSection settings={settings} update={update} />
            <SoundSection settings={settings} update={update} />
            <FlowBarSection settings={settings} update={update} />
            <NotificationsSection settings={settings} update={update} />
          </>
        );
        break;
      case "privacy":
        content = <PrivacySection settings={settings} update={update} />;
        break;
      case "speech":
        content = (
          <SpeechEngineSection
            settings={settings}
            options={options}
            keys={keys}
            onKeys={setKeys}
            update={update}
          />
        );
        break;
      case "ai":
        content = (
          <AiProviderSection
            settings={settings}
            options={options}
            keys={keys}
            onKeys={setKeys}
            update={update}
          />
        );
        break;
      case "experimental":
        content = (
          <ExperimentalSection
            settings={settings}
            commandShortcut={commandShortcut}
            reveal={reveal("experimental")}
            update={update}
            onCommandModeOn={() => setIntro(true)}
          />
        );
        break;
      case "vibeCoding":
        content = <VibeCodingSection settings={settings} update={update} />;
        break;
    }
  }

  const problem = error ?? loadError;
  return (
    <Page
      title={t("settings.title")}
      wide
      modal={
        intro && (
          <CommandModeIntro
            shortcut={commandShortcut}
            onDone={() => closeIntro(true)}
            onDisable={() => closeIntro(false)}
          />
        )
      }
    >
      <div className="settings-layout grid grid-cols-[10.5rem_minmax(0,1fr)] items-start gap-7 max-[800px]:grid-cols-1 max-[800px]:gap-5">
        <div
          className="settings-nav sticky top-(--toolbar-h) flex flex-col gap-0.5 max-[800px]:static max-[800px]:flex-row max-[800px]:flex-wrap max-[800px]:gap-1"
          role="tablist"
          aria-orientation="vertical"
          aria-label={t("settings.sections")}
          onKeyDown={onTabKeyDown}
        >
          {SETTINGS_SECTIONS.map((id) => {
            const Icon = SECTION_ICONS[id];
            return (
              <button
                key={id}
                id={`settings-tab-${id}`}
                type="button"
                role="tab"
                className="settings-tab group flex min-h-7 items-center gap-2 rounded-md px-2 py-1 text-left text-base leading-tight text-foreground/90 transition-colors duration-100 outline-none hover:bg-accent focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring/70 aria-selected:bg-foreground/[0.07] aria-selected:font-medium aria-selected:text-foreground max-[800px]:min-h-6 max-[800px]:gap-1.5 max-[800px]:rounded-full max-[800px]:border max-[800px]:bg-card max-[800px]:px-2.5 max-[800px]:py-0.5 max-[800px]:text-sm max-[800px]:aria-selected:border-transparent windows:min-h-9"
                aria-selected={id === section}
                aria-controls="settings-panel"
                tabIndex={id === section ? 0 : -1}
                onClick={() => setSection(id)}
              >
                <Icon
                  aria-hidden="true"
                  strokeWidth={1.75}
                  className="size-4 shrink-0 text-muted-foreground group-aria-selected:text-brand max-[800px]:size-3.5"
                />
                {t(`settings.${id}`)}
              </button>
            );
          })}
          <p className="settings-hint mt-3 px-2 text-xs text-muted-foreground max-[800px]:hidden">
            {t("settings.pageHint")}
          </p>
        </div>
        <div
          id="settings-panel"
          className="settings-panel flex min-w-0 flex-col gap-7"
          role="tabpanel"
          aria-labelledby={`settings-tab-${section}`}
        >
          {problem && (
            <p className="text-sm text-destructive" role="alert">
              {problem}
            </p>
          )}
          {content ?? (!problem && <p className="text-muted-foreground">{t("common.loading")}</p>)}
        </div>
      </div>
    </Page>
  );
}
