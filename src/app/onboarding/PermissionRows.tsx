import { CircleCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Row, RowText } from "@/components/settings";
import { Button } from "@/components/ui/button";
import {
  openPermissionSettings,
  type Permission,
  type PermissionsInfo,
  type PermissionStatus,
} from "../../lib/ipc";
import { IS_WINDOWS } from "../../lib/platform";

/** Windows asks no Accessibility permission. */
const ROWS: Permission[] = IS_WINDOWS ? ["microphone"] : ["microphone", "accessibility"];

interface PermissionRowsProps {
  info: PermissionsInfo | null;
}

/** Microphone and, on macOS, Accessibility with their live state and a way to grant each (ONB-01, ONB-08). */
export function PermissionRows({ info }: PermissionRowsProps) {
  const { t } = useTranslation();
  const state = (status: PermissionStatus | undefined) =>
    status ? t(`onboarding.permission.${status}`) : t("common.loading");
  return (
    <ul className="permission-rows m-0 flex list-none flex-col divide-y divide-separator rounded-xl border bg-card p-0 shadow-xs">
      {ROWS.map((permission) => {
        const status = info?.[permission];
        return (
          <li key={permission}>
            <Row>
              <RowText
                title={t(`onboarding.permission.${permission}`)}
                description={t(`onboarding.permission.${permission}Why`)}
              />
              <div className="flex items-center gap-3">
                <span
                  className={
                    status === "granted"
                      ? "flex items-center gap-1 text-sm text-success"
                      : "text-sm text-muted-foreground"
                  }
                  aria-live="polite"
                >
                  {status === "granted" && <CircleCheck aria-hidden="true" className="size-3.5" />}
                  {state(status)}
                </span>
                {status && status !== "granted" && (
                  <Button
                    size="sm"
                    variant="default"
                    onClick={() => void openPermissionSettings(permission).catch(() => undefined)}
                  >
                    {permission === "microphone" && status === "notDetermined"
                      ? t("onboarding.permission.grant")
                      : t("onboarding.permission.openSettings")}
                  </Button>
                )}
              </div>
            </Row>
          </li>
        );
      })}
    </ul>
  );
}
