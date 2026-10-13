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
      <div className="settings-layout">
        <div
          className="settings-nav"
          role="tablist"
          aria-orientation="vertical"
          aria-label={t("settings.sections")}
          onKeyDown={onTabKeyDown}
        >
          {SETTINGS_SECTIONS.map((id) => (
            <button
              key={id}
              id={`settings-tab-${id}`}
              type="button"
              role="tab"
              className="settings-tab"
              aria-selected={id === section}
              aria-controls="settings-panel"
              tabIndex={id === section ? 0 : -1}
              onClick={() => setSection(id)}
            >
              {t(`settings.${id}`)}
            </button>
          ))}
          <p className="muted settings-hint">{t("settings.pageHint")}</p>
        </div>
        <div
          id="settings-panel"
          className="settings-panel"
          role="tabpanel"
          aria-labelledby={`settings-tab-${section}`}
        >
          {problem && (
            <p className="error" role="alert">
              {problem}
            </p>
          )}
          {content ?? (!problem && <p className="muted">{t("common.loading")}</p>)}
        </div>
      </div>
    </Page>
  );
}
