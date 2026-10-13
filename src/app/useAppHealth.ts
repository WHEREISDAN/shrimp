import { useEffect, useState } from "react";
import { EVENTS, getAppHealth, type AppHealth } from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";

/** What the error screens need, kept current (APP-09). */
export function useAppHealth() {
  const [health, setHealth] = useState<AppHealth | null>(null);
  useEffect(() => {
    getAppHealth()
      .then(setHealth)
      .catch(() => undefined);
  }, []);
  useTauriEvent<AppHealth>(EVENTS.health, setHealth);
  return health;
}
