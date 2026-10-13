import { useTranslation } from "react-i18next";
import type { Settings, SettingsPatch } from "../lib/ipc";
import { ToggleRow } from "./ToggleRow";

interface NotificationsSectionProps {
  settings: Settings;
  update: (patch: SettingsPatch) => Promise<void>;
}

/** Settings › Notifications (NOT-01). */
export function NotificationsSection({ settings, update }: NotificationsSectionProps) {
  const { t } = useTranslation();
  const { notifications } = settings;
  return (
    <section className="card stack" aria-labelledby="notifications-heading">
      <h2 id="notifications-heading">{t("notifications.title")}</h2>
      <ToggleRow
        label={t("notifications.suggestions")}
        description={t("notifications.suggestionsBody")}
        checked={notifications.suggestions}
        onChange={(suggestions) => void update({ notifications: { suggestions } })}
      />
      <ToggleRow
        label={t("notifications.announcements")}
        description={t("notifications.announcementsBody")}
        checked={notifications.announcements}
        onChange={(announcements) => void update({ notifications: { announcements } })}
      />
      <ToggleRow
        label={t("notifications.milestones")}
        description={t("notifications.milestonesBody")}
        checked={notifications.milestones}
        onChange={(milestones) => void update({ notifications: { milestones } })}
      />
      <p className="muted">{t("notifications.alwaysShown")}</p>
    </section>
  );
}
