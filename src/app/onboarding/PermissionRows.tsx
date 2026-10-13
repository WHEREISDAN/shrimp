import { useTranslation } from "react-i18next";
import {
  openPermissionSettings,
  type Permission,
  type PermissionsInfo,
  type PermissionStatus,
} from "../../lib/ipc";

const ROWS: Permission[] = ["microphone", "accessibility"];

interface PermissionRowsProps {
  info: PermissionsInfo | null;
}

/** Microphone and Accessibility with their live state and a way to grant each (ONB-01, ONB-08). */
export function PermissionRows({ info }: PermissionRowsProps) {
  const { t } = useTranslation();
  const state = (status: PermissionStatus | undefined) =>
    status ? t(`onboarding.permission.${status}`) : t("common.loading");
  return (
    <ul className="rows permission-rows">
      {ROWS.map((permission) => {
        const status = info?.[permission];
        return (
          <li key={permission} className="row">
            <div>
              <p className="card-title">{t(`onboarding.permission.${permission}`)}</p>
              <p className="muted">{t(`onboarding.permission.${permission}Why`)}</p>
            </div>
            <div className="row-end">
              <span className={status === "granted" ? "ok" : "muted"} aria-live="polite">
                {status === "granted" && "✓ "}
                {state(status)}
              </span>
              {status && status !== "granted" && (
                <button
                  type="button"
                  className="primary"
                  onClick={() => void openPermissionSettings(permission).catch(() => undefined)}
                >
                  {permission === "microphone" && status === "notDetermined"
                    ? t("onboarding.permission.grant")
                    : t("onboarding.permission.openSettings")}
                </button>
              )}
            </div>
          </li>
        );
      })}
    </ul>
  );
}
