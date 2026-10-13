import { useCallback, useEffect, useState } from "react";
import {
  errorMessage,
  EVENTS,
  getSettings,
  updateSettings,
  type Settings,
  type SettingsPatch,
} from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";

export interface SettingsStore {
  settings: Settings | null;
  error: string | null;
  /** Saves a patch and shows what the backend stored. */
  update: (patch: SettingsPatch) => Promise<void>;
  /** Shows settings another command saved. */
  apply: (settings: Settings) => void;
}

/** The saved settings, kept current when they change elsewhere. */
export function useSettings(): SettingsStore {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    getSettings()
      .then((loaded) => !cancelled && setSettings(loaded))
      .catch((err: unknown) => !cancelled && setError(errorMessage(err)));
    return () => {
      cancelled = true;
    };
  }, []);
  useTauriEvent<Settings>(EVENTS.settings, setSettings);

  const update = useCallback(async (patch: SettingsPatch) => {
    try {
      setSettings(await updateSettings(patch));
      setError(null);
    } catch (err) {
      setError(errorMessage(err));
    }
  }, []);

  return { settings, error, update, apply: setSettings };
}
