import { useEffect, useState } from "react";
import {
  getPermissions,
  openPermissionSettings,
  type Permission,
  type PermissionsInfo,
  type PermissionStatus,
} from "../lib/ipc";

/** Grants change in System Settings, outside the app, so they are polled. */
const POLL_MS = 2000;

const ROWS: { permission: Permission; name: string; purpose: string }[] = [
  { permission: "microphone", name: "Microphone", purpose: "Hears you while you dictate." },
  {
    permission: "accessibility",
    name: "Accessibility",
    purpose: "Listens for the dictation shortcut and pastes the text.",
  },
];

const STATUS_TEXT: Record<PermissionStatus, string> = {
  granted: "Allowed",
  denied: "Not allowed",
  notDetermined: "Not asked yet",
};

export function PermissionsCard() {
  const [info, setInfo] = useState<PermissionsInfo | null>(null);

  useEffect(() => {
    let cancelled = false;
    const refresh = () =>
      getPermissions()
        .then((next) => !cancelled && setInfo(next))
        .catch(() => undefined);
    void refresh();
    const timer = setInterval(() => void refresh(), POLL_MS);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, []);

  return (
    <section className="card" aria-labelledby="permissions-heading">
      <h2 id="permissions-heading">Permissions</h2>
      <ul className="rows">
        {ROWS.map(({ permission, name, purpose }) => {
          const status = info?.[permission];
          return (
            <li key={permission} className="row">
              <div>
                <p className="card-title">{name}</p>
                <p className="muted">{purpose}</p>
              </div>
              <div className="row-end">
                <span className={status === "granted" ? "ok" : "muted"}>
                  {status ? STATUS_TEXT[status] : "…"}
                </span>
                {status && status !== "granted" && (
                  <button
                    type="button"
                    onClick={() => void openPermissionSettings(permission).catch(() => undefined)}
                  >
                    {status === "notDetermined" ? "Allow…" : "Open Settings"}
                  </button>
                )}
              </div>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
