import { ShieldAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { DragBand } from "@/components/drag-band";
import { Button } from "@/components/ui/button";
import type { PermissionsInfo } from "../../lib/ipc";
import { PermissionRows } from "./PermissionRows";

interface ReverifyScreenProps {
  info: PermissionsInfo;
  onLater: () => void;
}

/** A permission granted before is missing, usually after an update (ONB-08). */
export function ReverifyScreen({ info, onLater }: ReverifyScreenProps) {
  const { t } = useTranslation();
  return (
    <main
      className="onboarding flex min-h-full flex-col items-center justify-center overflow-y-auto bg-background px-8 py-16"
      aria-labelledby="reverify-title"
    >
      <DragBand />
      <section className="onboarding-card flex w-full max-w-[32rem] animate-pop-in flex-col gap-4">
        <span
          aria-hidden="true"
          className="grid size-12 place-items-center rounded-2xl bg-warning/12 text-warning"
        >
          <ShieldAlert className="size-6" strokeWidth={1.75} />
        </span>
        <h1 id="reverify-title" className="font-display text-2xl font-bold tracking-[-0.01em]">
          {t("onboarding.reverifyTitle")}
        </h1>
        <p className="text-muted-foreground">{t("onboarding.reverifyBody")}</p>
        <PermissionRows info={info} />
        <p className="text-sm text-muted-foreground">{t("onboarding.reverifyHint")}</p>
        <div className="onboarding-actions flex justify-end">
          <Button size="lg" onClick={onLater}>
            {t("onboarding.later")}
          </Button>
        </div>
      </section>
    </main>
  );
}
