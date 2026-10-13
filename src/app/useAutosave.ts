import { useEffect, useRef } from "react";

/** Calls `save` with `value` once it has stopped changing for `delayMs`, and on unmount. */
export function useAutosave<T>(value: T, dirty: boolean, save: (value: T) => void, delayMs = 700) {
  const latest = useRef({ value, dirty, save });
  useEffect(() => {
    latest.current = { value, dirty, save };
  });
  useEffect(() => {
    if (!dirty) {
      return;
    }
    const timer = setTimeout(() => save(value), delayMs);
    return () => clearTimeout(timer);
  }, [value, dirty, save, delayMs]);
  useEffect(
    () => () => {
      if (latest.current.dirty) {
        latest.current.save(latest.current.value);
      }
    },
    [],
  );
}
