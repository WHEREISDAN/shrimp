import { useTranslation } from "react-i18next";
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
    <main className="onboarding" aria-labelledby="reverify-title">
      <section className="onboarding-card">
        <h1 id="reverify-title">{t("onboarding.reverifyTitle")}</h1>
        <p>{t("onboarding.reverifyBody")}</p>
        <PermissionRows info={info} />
        <p className="muted">{t("onboarding.reverifyHint")}</p>
        <div className="onboarding-actions">
          <button type="button" onClick={onLater}>
            {t("onboarding.later")}
          </button>
        </div>
      </section>
    </main>
  );
}
