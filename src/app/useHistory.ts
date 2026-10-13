import { useCallback, useEffect, useRef, useState } from "react";
import { errorMessage, EVENTS, listHistory, type HistoryList } from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";

/** Waits this long after typing before searching. */
const SEARCH_DELAY_MS = 150;

/** The history matching `query`, kept current as dictations arrive. */
export function useHistory(query: string) {
  const [list, setList] = useState<HistoryList | null>(null);
  const [error, setError] = useState<string | null>(null);
  const queryRef = useRef(query);
  const requestRef = useRef(0);

  const load = useCallback((search: string) => {
    const request = ++requestRef.current;
    listHistory(search)
      .then((next) => {
        if (request === requestRef.current) {
          setList(next);
          setError(null);
        }
      })
      .catch((err: unknown) => request === requestRef.current && setError(errorMessage(err)));
  }, []);

  useEffect(() => {
    queryRef.current = query;
    const timer = setTimeout(() => load(query), query ? SEARCH_DELAY_MS : 0);
    return () => clearTimeout(timer);
  }, [query, load]);

  useTauriEvent(EVENTS.history, () => load(queryRef.current));
  return { list, error };
}
