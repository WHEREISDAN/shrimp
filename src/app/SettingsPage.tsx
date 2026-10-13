import { useEffect, useState } from "react";
import { errorMessage, getAiOptions, getApiKeys, type AiOptions, type ApiKeys } from "../lib/ipc";
import { AiProviderSection } from "./AiProviderSection";
import { Page } from "./Page";
import { SpeechEngineSection } from "./SpeechEngineSection";
import { useSettings } from "./useSettings";

export function SettingsPage() {
  const { settings, error, update } = useSettings();
  const [options, setOptions] = useState<AiOptions | null>(null);
  const [keys, setKeys] = useState<ApiKeys | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

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
    return () => {
      cancelled = true;
    };
  }, []);

  const problem = error ?? loadError;
  return (
    <Page title="Settings">
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
        </>
      ) : (
        !problem && <p className="muted">Loading…</p>
      )}
    </Page>
  );
}
