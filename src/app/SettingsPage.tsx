import { useEffect, useState } from "react";
import {
  errorMessage,
  getAiOptions,
  getApiKeys,
  getShortcuts,
  type AiOptions,
  type ApiKeys,
} from "../lib/ipc";
import { AiProviderSection } from "./AiProviderSection";
import { CommandModeIntro } from "./CommandModeIntro";
import { ExperimentalSection } from "./ExperimentalSection";
import { Page } from "./Page";
import { PrivacySection } from "./PrivacySection";
import { ShortcutsSection } from "./ShortcutsSection";
import { SpeechEngineSection } from "./SpeechEngineSection";
import { useSettings } from "./useSettings";

interface SettingsPageProps {
  /** Increments each time Settings › Experimental should come into view. */
  revealExperimental?: number;
}

export function SettingsPage({ revealExperimental = 0 }: SettingsPageProps) {
  const { settings, error, update, apply } = useSettings();
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
        const command = list.find((shortcut) => shortcut.action === "commandMode");
        if (!cancelled && command) {
          setCommandShortcut(command.shortcut);
        }
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);

  const closeIntro = (keepOn: boolean) => {
    setIntro(false);
    void update({
      experimental: keepOn
        ? { commandModeIntroSeen: true }
        : { commandModeIntroSeen: true, commandMode: false },
    });
  };

  const problem = error ?? loadError;
  return (
    <Page
      title="Settings"
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
      {problem && (
        <p className="error" role="alert">
          {problem}
        </p>
      )}
      {settings && options && keys ? (
        <>
          <AiProviderSection
            settings={settings}
            options={options}
            keys={keys}
            onKeys={setKeys}
            update={update}
          />
          <SpeechEngineSection
            settings={settings}
            options={options}
            keys={keys}
            onKeys={setKeys}
            update={update}
          />
          <ShortcutsSection settings={settings} onSaved={apply} />
          <PrivacySection settings={settings} update={update} />
          <ExperimentalSection
            settings={settings}
            commandShortcut={commandShortcut}
            reveal={revealExperimental}
            update={update}
            onCommandModeOn={() => setIntro(true)}
          />
        </>
      ) : (
        !problem && <p className="muted">Loading…</p>
      )}
    </Page>
  );
}
