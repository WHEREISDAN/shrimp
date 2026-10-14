import type { TFunction } from "i18next";
import { useCallback, useEffect, useRef, useState } from "react";
import { ChevronDown, ChevronUp } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Row, Section } from "@/components/settings";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
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
    <Section
      id="microphone"
      headingId="microphone-heading"
      headingRef={headingRef}
      focusableHeading
      title={t("microphone.title")}
    >
      <div className="flex flex-col divide-y divide-separator rounded-xl border bg-card shadow-xs">
        {list && (
          <fieldset className="m-0 flex min-w-0 flex-col divide-y divide-separator p-0">
            <legend className="sr-only">{t("microphone.title")}</legend>
            <label className="flex min-h-(--row-min-h) items-start gap-3 px-4 py-3">
              <input
                type="radio"
                name="microphone"
                className="mt-0.5"
                checked={list.selected === null}
                onChange={() => run(selectMicrophone(null))}
              />
              <span className="flex flex-col gap-0.5">
                <span>{t("microphone.autoDetect")}</span>
                <span className="text-sm text-muted-foreground">
                  {t("microphone.autoDetectHint")}
                </span>
              </span>
            </label>
            <ul
              className="list m-0 flex list-none flex-col divide-y divide-separator p-0 empty:hidden"
              aria-label={t("microphone.list")}
            >
              {shown.map((device) => {
                const usable = device.connected && device.unavailable === null;
                const position = ranked.indexOf(device);
                return (
                  <li
                    key={device.id}
                    className={cn(
                      "list-row flex min-h-(--row-min-h) flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2.5",
                      !usable && "dimmed",
                    )}
                  >
                    <label className="list-main flex min-w-0 flex-1 basis-48 items-start gap-3">
                      <input
                        type="radio"
                        name="microphone"
                        className="mt-0.5"
                        disabled={!usable}
                        checked={list.selected === device.id}
                        onChange={() => run(selectMicrophone(device.id))}
                      />
                      <span className="flex min-w-0 flex-col gap-0.5">
                        <span className={cn("list-title", !usable && "text-muted-foreground")}>
                          {deviceLabel(device, t)}
                        </span>
                        <span className="list-meta text-sm text-muted-foreground">
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
                      <div className="row-actions flex items-center gap-1">
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          aria-label={t("microphone.moveUp", { name: device.name })}
                          disabled={position <= 0}
                          onClick={() => run(rankMicrophone(device.id, true))}
                        >
                          <ChevronUp aria-hidden="true" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          aria-label={t("microphone.moveDown", { name: device.name })}
                          disabled={position === ranked.length - 1}
                          onClick={() => run(rankMicrophone(device.id, false))}
                        >
                          <ChevronDown aria-hidden="true" />
                        </Button>
                        {!device.connected && (
                          <Button
                            size="sm"
                            aria-label={t("microphone.forgetDevice", { name: device.name })}
                            onClick={() => run(forgetMicrophone(device.id))}
                          >
                            {t("microphone.forget")}
                          </Button>
                        )}
                      </div>
                    )}
                  </li>
                );
              })}
            </ul>
            {shown.length === 0 && (
              <p className="px-4 py-3 text-muted-foreground">{t("microphone.none")}</p>
            )}
            <label className="flex min-h-(--row-min-h) items-start gap-3 px-4 py-3">
              <input
                type="checkbox"
                className="mt-0.5"
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
              <span className="flex flex-col gap-0.5">
                <span>{t("microphone.showOthers")}</span>
                <span className="text-sm text-muted-foreground">
                  {t("microphone.showOthersHint")}
                </span>
              </span>
            </label>
          </fieldset>
        )}
        <Row className="flex-nowrap">
          <span className="shrink-0">{t("microphone.inputLevel")}</span>
          <meter
            className="level-meter h-1.5 w-full max-w-64 flex-1"
            aria-label={t("microphone.level")}
            min={0}
            max={1}
            value={active ? level : 0}
          />
        </Row>
      </div>
      {error && (
        <p className="px-1 text-sm text-destructive" role="alert">
          {error}
        </p>
      )}
      <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 px-1">
        <p className="text-sm text-muted-foreground">{t("microphone.notSure")}</p>
        <Button size="sm" onClick={() => setAdvice(true)}>
          {t("micAdvice.title")}
        </Button>
      </div>
      {advice && <MicAdviceDialog onClose={() => setAdvice(false)} />}
    </Section>
  );
}
