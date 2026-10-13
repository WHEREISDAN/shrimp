import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  getPermissions,
  openPermissionSettings,
  type Permission,
  type PermissionsInfo,
  type PermissionStatus,
} from "../lib/ipc";
import { IS_WINDOWS } from "../lib/platform";

/** Grants change in System Settings, outside the app, so they are polled. */
const POLL_MS = 2000;

const ALL_ROWS = [
  {
    permission: "microphone",
    name: "onboarding.permission.microphone",
    purpose: "permissionsCard.microphoneWhy",
  },
  {
    permission: "accessibility",
    name: "onboarding.permission.accessibility",
    purpose: "permissionsCard.accessibilityWhy",
  },
] as const satisfies readonly { permission: Permission; name: string; purpose: string }[];

/** Windows asks no Accessibility permission. */
const ROWS = ALL_ROWS.filter((row) => !IS_WINDOWS || row.permission === "microphone");

const STATUS_TEXT = {
  granted: "onboarding.permission.granted",
  denied: "onboarding.permission.denied",
  notDetermined: "onboarding.permission.notDetermined",
} as const satisfies Record<PermissionStatus, string>;

interface PermissionsCardProps {
  /** Home shows the card only while a permission is missing. */
  hideWhenGranted?: boolean;
}

export function PermissionsCard({ hideWhenGranted = false }: PermissionsCardProps) {
  const { t } = useTranslation();
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

  if (hideWhenGranted && info?.microphone === "granted" && info.accessibility === "granted") {
    return null;
  }
  return (
    <section className="card" aria-labelledby="permissions-heading">
      <h2 id="permissions-heading">{t("permissionsCard.title")}</h2>
      <ul className="rows">
        {ROWS.map(({ permission, name, purpose }) => {
          const status = info?.[permission];
          return (
            <li key={permission} className="row">
              <div>
                <p className="card-title">{t(name)}</p>
                <p className="muted">{t(purpose)}</p>
              </div>
              <div className="row-end">
                <span className={status === "granted" ? "ok" : "muted"}>
                  {status ? t(STATUS_TEXT[status]) : "…"}
                </span>
                {status && status !== "granted" && (
                  <button
                    type="button"
                    onClick={() => void openPermissionSettings(permission).catch(() => undefined)}
                  >
                    {status === "notDetermined"
                      ? t("permissionsCard.allow")
                      : t("permissionsCard.openSettings")}
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
