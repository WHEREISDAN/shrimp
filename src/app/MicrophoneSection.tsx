import { useCallback, useEffect, useRef, useState } from "react";
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

function deviceLabel(device: MicrophoneEntry): string {
  return device.kind === "builtIn" ? `${device.name} (built-in, recommended)` : device.name;
}

/**
 * Settings › Microphone (MIC-02, MIC-03, MIC-05, MIC-06): Auto-detect or a microphone, the
 * ranking Auto-detect follows, and a live level meter for the microphone in use.
 */
export function MicrophoneSection({ reveal = 0 }: MicrophoneSectionProps) {
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

  // The meter follows the microphone in use, so it restarts when that changes.
  const active = list?.active ?? null;
  useEffect(() => {
    if (!active) {
      return;
    }
    void startMicPreview().catch(() => undefined);
    return () => void stopMicPreview().catch(() => undefined);
  }, [active]);

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
        Microphone
      </h2>
      {list && (
        <fieldset className="plain-fieldset stack">
          <legend className="sr-only">Microphone</legend>
          <label className="check">
            <input
              type="radio"
              name="microphone"
              checked={list.selected === null}
              onChange={() => run(selectMicrophone(null))}
            />
            <span>
              Auto-detect
              <span className="muted"> · uses the highest-ranked microphone connected</span>
            </span>
          </label>
          <ul className="list" aria-label="Microphones">
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
                      <span className="list-title">{deviceLabel(device)}</span>
                      <span className="list-meta">
                        {!device.connected
                          ? "Not connected"
                          : (device.unavailable ??
                            (device.id === list.active ? "In use" : "Connected"))}
                        {device.kind === "bluetooth" ? " · Bluetooth" : ""}
                      </span>
                    </span>
                  </label>
                  {device.rank !== null && (
                    <div className="row-actions">
                      <button
                        type="button"
                        aria-label={`Move ${device.name} up`}
                        disabled={position <= 0}
                        onClick={() => run(rankMicrophone(device.id, true))}
                      >
                        ↑
                      </button>
                      <button
                        type="button"
                        aria-label={`Move ${device.name} down`}
                        disabled={position === ranked.length - 1}
                        onClick={() => run(rankMicrophone(device.id, false))}
                      >
                        ↓
                      </button>
                      {!device.connected && (
                        <button
                          type="button"
                          aria-label={`Forget ${device.name}`}
                          onClick={() => run(forgetMicrophone(device.id))}
                        >
                          Forget
                        </button>
                      )}
                    </div>
                  )}
                </li>
              );
            })}
          </ul>
          {shown.length === 0 && <p className="muted">No microphone detected.</p>}
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
              Show other devices{" "}
              <span className="muted">
                (virtual and loopback inputs, never chosen automatically)
              </span>
            </span>
          </label>
        </fieldset>
      )}
      <div className="progress-row">
        <span className="muted">Input level</span>
        <meter
          className="level-meter"
          aria-label="Microphone level"
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
        <p className="muted">Not sure which microphone to use?</p>
        <button type="button" onClick={() => setAdvice(true)}>
          Find the right mic for you
        </button>
      </div>
      {advice && <MicAdviceDialog onClose={() => setAdvice(false)} />}
    </section>
  );
}
