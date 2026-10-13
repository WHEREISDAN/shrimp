import { useTranslation } from "react-i18next";
import type { Settings } from "../lib/ipc";

/** Says that Privacy mode overrides the cloud choices on this page (PRIV-03). */
export function PrivacyModeNote({ settings }: { settings: Settings }) {
  const { t } = useTranslation();
  if (!settings.privacy.privacyMode) {
    return null;
  }
  return (
    <p className="banner" role="note">
      {t("privacy.privacyModeOn")}
    </p>
  );
}
