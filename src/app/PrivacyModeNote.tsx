import { Info } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { Settings } from "../lib/ipc";

/** Says that Privacy mode overrides the cloud choices on this page (PRIV-03). */
export function PrivacyModeNote({ settings }: { settings: Settings }) {
  const { t } = useTranslation();
  if (!settings.privacy.privacyMode) {
    return null;
  }
  return (
    <p
      className="flex items-start gap-2.5 rounded-lg border bg-muted/50 px-3.5 py-2.5 text-sm"
      role="note"
    >
      <Info aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
      <span>{t("privacy.privacyModeOn")}</span>
    </p>
  );
}
