import { useEffect, useState } from "react";
import { CircleCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Group, Row, RowText, Section } from "@/components/settings";
import { Button } from "@/components/ui/button";
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
    <Section headingId="permissions-heading" title={t("permissionsCard.title")}>
      <Group role="list">
        {ROWS.map(({ permission, name, purpose }) => {
          const status = info?.[permission];
          return (
            <Row key={permission} role="listitem">
              <RowText title={t(name)} description={t(purpose)} />
              <div className="flex items-center gap-3">
                <span
                  className={
                    status === "granted"
                      ? "flex items-center gap-1 text-sm text-success"
                      : "text-sm text-muted-foreground"
                  }
                >
                  {status === "granted" && <CircleCheck aria-hidden="true" className="size-3.5" />}
                  {status ? t(STATUS_TEXT[status]) : "…"}
                </span>
                {status && status !== "granted" && (
                  <Button
                    size="sm"
                    variant={status === "notDetermined" ? "default" : "outline"}
                    onClick={() => void openPermissionSettings(permission).catch(() => undefined)}
                  >
                    {status === "notDetermined"
                      ? t("permissionsCard.allow")
                      : t("permissionsCard.openSettings")}
                  </Button>
                )}
              </div>
            </Row>
          );
        })}
      </Group>
    </Section>
  );
}
