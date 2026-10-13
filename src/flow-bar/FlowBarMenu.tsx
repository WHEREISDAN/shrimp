import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  flowBarAction,
  getMicrophones,
  selectMicrophone,
  type FlowBarAction,
  type MicrophoneList,
} from "../lib/ipc";

interface FlowBarMenuProps {
  onClose: () => void;
}

/** The Flow Bar's right-click menu (BAR-06). */
export function FlowBarMenu({ onClose }: FlowBarMenuProps) {
  const { t } = useTranslation();
  const [microphones, setMicrophones] = useState<MicrophoneList | null>(null);
  const [showMicrophones, setShowMicrophones] = useState(false);

  useEffect(() => {
    let cancelled = false;
    getMicrophones()
      .then((list) => !cancelled && setMicrophones(list))
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);

  const run = (action: FlowBarAction) => {
    void flowBarAction(action).catch(() => undefined);
    onClose();
  };
  const choose = (id: string | null) => {
    void selectMicrophone(id).catch(() => undefined);
    onClose();
  };
  const connected = microphones?.devices.filter((device) => device.connected) ?? [];

  return (
    <div className="bar-menu" role="menu" aria-label={t("flowBarMenu.label")}>
      <button
        type="button"
        role="menuitem"
        aria-expanded={showMicrophones}
        onClick={() => setShowMicrophones(!showMicrophones)}
      >
        {t("flowBarMenu.microphone")}
      </button>
      {showMicrophones && (
        <div className="bar-submenu" role="group" aria-label={t("microphone.list")}>
          <button
            type="button"
            role="menuitemradio"
            aria-checked={microphones?.selected === null}
            onClick={() => choose(null)}
          >
            {t("microphone.autoDetect")}
          </button>
          {connected.map((device) => (
            <button
              key={device.id}
              type="button"
              role="menuitemradio"
              aria-checked={microphones?.selected === device.id}
              disabled={device.unavailable !== null}
              onClick={() => choose(device.id)}
            >
              {device.name}
            </button>
          ))}
        </div>
      )}
      <button type="button" role="menuitem" onClick={() => run("pasteLast")}>
        {t("flowBarMenu.pasteLast")}
      </button>
      <button type="button" role="menuitem" onClick={() => run("history")}>
        {t("flowBarMenu.history")}
      </button>
      <button type="button" role="menuitem" onClick={() => run("settings")}>
        {t("flowBarMenu.settings")}
      </button>
      <button type="button" role="menuitem" onClick={() => run("hide")}>
        {t("flowBarMenu.hide")}
      </button>
    </div>
  );
}
