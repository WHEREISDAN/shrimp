import { useEffect, useState } from "react";
import { getPermissions, type PermissionsInfo } from "../../lib/ipc";

/** Grants change in System Settings, outside the app, so they are polled (ONB-01). */
const POLL_MS = 1000;

/** The permissions, re-read every second and whenever the window comes forward. */
export function usePermissions() {
  const [info, setInfo] = useState<PermissionsInfo | null>(null);
  useEffect(() => {
    let cancelled = false;
    const refresh = () =>
      getPermissions()
        .then((next) => !cancelled && setInfo(next))
        .catch(() => undefined);
    void refresh();
    const timer = setInterval(() => void refresh(), POLL_MS);
    window.addEventListener("focus", refresh);
    return () => {
      cancelled = true;
      clearInterval(timer);
      window.removeEventListener("focus", refresh);
    };
  }, []);
  return info;
}
