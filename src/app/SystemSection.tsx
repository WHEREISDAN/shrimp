import { useState } from "react";
import { useTranslation } from "react-i18next";
import { resetApp, type Settings, type SettingsPatch } from "../lib/ipc";
import { ConfirmDialog } from "./ConfirmDialog";
import { ToggleRow } from "./ToggleRow";
import { IS_WINDOWS } from "../lib/platform";

interface SystemSectionProps {
  settings: Settings;
  update: (patch: SettingsPatch) => Promise<void>;
}

/** Settings › System: launch at login, the Dock icon (macOS only) and Reset & restart (APP-06..08). */
export function SystemSection({ settings, update }: SystemSectionProps) {
  const { t } = useTranslation();
  const [resetting, setResetting] = useState(false);
  return (
    <section className="card stack" aria-labelledby="system-heading">
      <h2 id="system-heading">{t("system.title")}</h2>
      <ToggleRow
        label={t("system.launchAtLogin")}
        description={t("system.launchAtLoginBody")}
        checked={settings.app.launchAtLogin}
        onChange={(launchAtLogin) => void update({ app: { launchAtLogin } })}
      />
      {!IS_WINDOWS && (
        <ToggleRow
          label={t("system.showInDock")}
          description={t("system.showInDockBody")}
          checked={settings.app.showInDock}
          onChange={(showInDock) => void update({ app: { showInDock } })}
        />
      )}
      <div className="toggle-row">
        <div className="toggle-text">
          <p className="card-title">{t("system.resetTitle")}</p>
          <p className="muted">{t("system.resetBody")}</p>
        </div>
        <button type="button" className="danger" onClick={() => setResetting(true)}>
          {t("system.reset")}
        </button>
      </div>
      {resetting && (
        <ConfirmDialog
          title={t("system.resetConfirmTitle")}
          message={t("system.resetConfirmBody")}
          confirmLabel={t("system.reset")}
          onCancel={() => setResetting(false)}
          onConfirm={resetApp}
        />
      )}
    </section>
  );
}
