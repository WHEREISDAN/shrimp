import { useEffect, useState } from "react";
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
    <div className="bar-menu" role="menu" aria-label="Flow Bar menu">
      <button
        type="button"
        role="menuitem"
        aria-expanded={showMicrophones}
        onClick={() => setShowMicrophones(!showMicrophones)}
      >
        Microphone ›
      </button>
      {showMicrophones && (
        <div className="bar-submenu" role="group" aria-label="Microphones">
          <button
            type="button"
            role="menuitemradio"
            aria-checked={microphones?.selected === null}
            onClick={() => choose(null)}
          >
            Auto-detect
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
        Paste last transcript
      </button>
      <button type="button" role="menuitem" onClick={() => run("history")}>
        Transcript history
      </button>
      <button type="button" role="menuitem" onClick={() => run("settings")}>
        Settings
      </button>
      <button type="button" role="menuitem" onClick={() => run("hide")}>
        Hide for 1 hour
      </button>
    </div>
  );
}
