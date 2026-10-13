import { useCallback, useEffect, useState } from "react";
import { errorMessage, EVENTS, getInsights, type Insights } from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";

/** Refreshes this often, so a new day shows without a dictation. */
const REFRESH_MS = 60 * 60 * 1000;

/** Insights, refreshed after each dictation and hourly. */
export function useInsights() {
  const [insights, setInsights] = useState<Insights | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    getInsights()
      .then((next) => {
        setInsights(next);
        setError(null);
      })
      .catch((err: unknown) => setError(errorMessage(err)));
  }, []);

  useEffect(() => {
    load();
    const timer = setInterval(load, REFRESH_MS);
    return () => clearInterval(timer);
  }, [load]);
  useTauriEvent(EVENTS.history, load);
  return { insights, error, reload: load };
}
