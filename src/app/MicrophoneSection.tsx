import type { TFunction } from "i18next";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  errorMessage,
  EVENTS,
  forgetMicrophone,
  getMicrophones,
  rankMicrophone,
  selectMicrophone,
  startMicPreview,
  stopMicPreview,
  type MicrophoneEntry,
  type MicrophoneList,
} from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";
import { MicAdviceDialog } from "./MicAdviceDialog";

interface MicrophoneSectionProps {
  /** Increments each time the section should scroll into view. */
  reveal?: number;
}

function deviceLabel(device: MicrophoneEntry, t: TFunction): string {
  return device.kind === "builtIn" ? t("microphone.builtIn", { name: device.name }) : device.name;
}

/**
 * Settings › Microphone (MIC-02, MIC-03, MIC-05, MIC-06): Auto-detect or a microphone, the
 * ranking Auto-detect follows, and a live level meter for the microphone in use.
 */
export function MicrophoneSection({ reveal = 0 }: MicrophoneSectionProps) {
  const { t } = useTranslation();
  const [list, setList] = useState<MicrophoneList | null>(null);
  const [showOthers, setShowOthers] = useState(false);
  const [level, setLevel] = useState(0);
  const [advice, setAdvice] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const headingRef = useRef<HTMLHeadingElement>(null);

  useEffect(() => {
    let cancelled = false;
    getMicrophones()
      .then((loaded) => !cancelled && setList(loaded))
      .catch((err: unknown) => !cancelled && setError(errorMessage(err)));
    return () => {
      cancelled = true;
    };
  }, []);
  useTauriEvent<MicrophoneList>(EVENTS.microphones, setList);
  useTauriEvent<number>(EVENTS.micLevel, setLevel);

  // The meter follows the microphone in use, so it restarts when that changes,
  // and runs only while the window is open: closing it to the tray stops it.
  const active = list?.active ?? null;
  const [windowShown, setWindowShown] = useState(true);
  useTauriEvent<boolean>(EVENTS.mainVisible, setWindowShown);
  useEffect(() => {
    if (!active || !windowShown) {
      return;
    }
    void startMicPreview().catch(() => undefined);
    return () => void stopMicPreview().catch(() => undefined);
  }, [active, windowShown]);

  useEffect(() => {
    if (reveal === 0) {
      return;
    }
    const frame = requestAnimationFrame(() => {
      headingRef.current?.scrollIntoView({ block: "start" });
      headingRef.current?.focus();
    });
    return () => cancelAnimationFrame(frame);
  }, [reveal]);

  const run = useCallback((work: Promise<MicrophoneList>) => {
    setError(null);
    work.then(setList).catch((err: unknown) => setError(errorMessage(err)));
  }, []);

  const physical = list?.devices.filter((device) => device.kind !== "virtual") ?? [];
  const virtual = list?.devices.filter((device) => device.kind === "virtual") ?? [];
  const ranked = physical.filter((device) => device.rank !== null);
  const shown = showOthers ? [...physical, ...virtual] : physical;

  return (
    <section className="card stack" aria-labelledby="microphone-heading" id="microphone">
      <h2 id="microphone-heading" ref={headingRef} tabIndex={-1}>
        {t("microphone.title")}
      </h2>
      {list && (
        <fieldset className="plain-fieldset stack">
          <legend className="sr-only">{t("microphone.title")}</legend>
          <label className="check">
            <input
              type="radio"
              name="microphone"
              checked={list.selected === null}
              onChange={() => run(selectMicrophone(null))}
            />
            <span>
              {t("microphone.autoDetect")}
              <span className="muted"> · {t("microphone.autoDetectHint")}</span>
            </span>
          </label>
          <ul className="list" aria-label={t("microphone.list")}>
            {shown.map((device) => {
              const usable = device.connected && device.unavailable === null;
              const position = ranked.indexOf(device);
              return (
                <li key={device.id} className={usable ? "list-row" : "list-row dimmed"}>
                  <label className="list-main check">
                    <input
                      type="radio"
                      name="microphone"
                      disabled={!usable}
                      checked={list.selected === device.id}
                      onChange={() => run(selectMicrophone(device.id))}
                    />
                    <span>
                      <span className="list-title">{deviceLabel(device, t)}</span>
                      <span className="list-meta">
                        {!device.connected
                          ? t("microphone.notConnected")
                          : (device.unavailable ??
                            (device.id === list.active
                              ? t("microphone.inUse")
                              : t("microphone.connected")))}
                        {device.kind === "bluetooth" ? " · Bluetooth" : ""}
                      </span>
                    </span>
                  </label>
                  {device.rank !== null && (
                    <div className="row-actions">
                      <button
                        type="button"
                        aria-label={t("microphone.moveUp", { name: device.name })}
                        disabled={position <= 0}
                        onClick={() => run(rankMicrophone(device.id, true))}
                      >
                        ↑
                      </button>
                      <button
                        type="button"
                        aria-label={t("microphone.moveDown", { name: device.name })}
                        disabled={position === ranked.length - 1}
                        onClick={() => run(rankMicrophone(device.id, false))}
                      >
                        ↓
                      </button>
                      {!device.connected && (
                        <button
                          type="button"
                          aria-label={t("microphone.forgetDevice", { name: device.name })}
                          onClick={() => run(forgetMicrophone(device.id))}
                        >
                          {t("microphone.forget")}
                        </button>
                      )}
                    </div>
                  )}
                </li>
              );
            })}
          </ul>
          {shown.length === 0 && <p className="muted">{t("microphone.none")}</p>}
          <label className="check">
            <input
              type="checkbox"
              checked={showOthers}
              onChange={(event) => {
                const show = event.target.checked;
                setShowOthers(show);
                const selected = virtual.some((device) => device.id === list.selected);
                if (!show && selected) {
                  run(selectMicrophone(null));
                }
              }}
            />
            <span>
              {t("microphone.showOthers")}{" "}
              <span className="muted">{t("microphone.showOthersHint")}</span>
            </span>
          </label>
        </fieldset>
      )}
      <div className="progress-row">
        <span className="muted">{t("microphone.inputLevel")}</span>
        <meter
          className="level-meter"
          aria-label={t("microphone.level")}
          min={0}
          max={1}
          value={active ? level : 0}
        />
      </div>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <div className="row">
        <p className="muted">{t("microphone.notSure")}</p>
        <button type="button" onClick={() => setAdvice(true)}>
          {t("micAdvice.title")}
        </button>
      </div>
      {advice && <MicAdviceDialog onClose={() => setAdvice(false)} />}
    </section>
  );
}
