import { useEffect, useRef, useState } from "react";
import { Info } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Group, Note, Section } from "@/components/settings";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import {
  EVENTS,
  generateVoiceProfile,
  getVoiceProfile,
  type LlmProviderId,
  type VoiceProfileError,
  type VoiceProfileStatus,
} from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";

const PROVIDER_NAMES: Record<LlmProviderId, string> = {
  anthropic: "Anthropic",
  openai: "OpenAI",
  groq: "Groq",
  ollama: "Ollama",
};

/** A label and its value on one line of the profile's two columns. */
const detailRow = (className?: string) =>
  cn("col-span-full grid grid-cols-subgrid items-baseline gap-y-0.5", className);

interface VoiceProfileCardProps {
  provider: LlmProviderId;
  /** Increments each time the card should scroll into view and take focus. */
  reveal?: number;
}

/** Insights › Your voice (STA-04). */
export function VoiceProfileCard({ provider, reveal = 0 }: VoiceProfileCardProps) {
  const { t, i18n } = useTranslation();
  const [status, setStatus] = useState<VoiceProfileStatus | null>(null);
  const [building, setBuilding] = useState(false);
  const [problem, setProblem] = useState<VoiceProfileError | null>(null);
  const headingRef = useRef<HTMLHeadingElement>(null);

  const load = () => {
    getVoiceProfile()
      .then(setStatus)
      .catch(() => undefined);
  };
  useEffect(load, []);
  useTauriEvent(EVENTS.history, load);

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

  const generate = () => {
    setBuilding(true);
    setProblem(null);
    const language =
      new Intl.DisplayNames(["en"], { type: "language" }).of(i18n.language) ?? "English";
    generateVoiceProfile(language)
      .then(setStatus)
      .catch((err: unknown) =>
        setProblem(
          typeof err === "object" && err !== null && "kind" in err
            ? (err as VoiceProfileError)
            : { kind: "failed", message: String(err) },
        ),
      )
      .finally(() => setBuilding(false));
  };

  const profile = status?.profile ?? null;
  const ready = status ? status.totalWords >= status.nextAt : false;
  const remaining = status ? Math.max(0, status.nextAt - status.totalWords) : 0;
  return (
    <Section
      headingId="voice-heading"
      headingRef={headingRef}
      focusableHeading
      title={t("voice.title")}
    >
      <Group>
        {!profile && <p className="px-4 py-3.5 text-muted-foreground">{t("voice.intro")}</p>}
        {profile && (
          <div className="flex flex-col gap-3.5 px-4 py-4">
            <dl className="m-0 grid grid-cols-[max-content_minmax(0,1fr)] gap-x-8 gap-y-2.5">
              <div className="col-span-full flex flex-col gap-0.5">
                <dt className="text-sm font-medium text-muted-foreground">
                  {t("voice.archetype")}
                </dt>
                <dd className="m-0 font-display text-2xl font-semibold tracking-[-0.01em] text-brand-strong">
                  {profile.archetype}
                </dd>
                {profile.description && (
                  <dd className="m-0 mt-1 text-muted-foreground">{profile.description}</dd>
                )}
              </div>
              <div className={detailRow("mt-1.5 border-t border-separator pt-3.5")}>
                <dt className="text-muted-foreground">{t("voice.catchphrase")}</dt>
                <dd className="m-0 font-medium">“{profile.catchphrase}”</dd>
              </div>
              <div className={detailRow()}>
                <dt className="text-muted-foreground">{t("voice.mostCorrected")}</dt>
                <dd className="m-0 font-medium">{profile.mostCorrected}</dd>
              </div>
              <div className={detailRow()}>
                <dt className="text-muted-foreground">{t("voice.activeTimes")}</dt>
                <dd className="m-0 font-medium">{profile.activeTimes}</dd>
              </div>
            </dl>
            <p className="text-sm text-muted-foreground">
              {t("voice.generatedOn", {
                date: new Intl.DateTimeFormat(i18n.language, { dateStyle: "medium" }).format(
                  new Date(profile.generatedAt),
                ),
                words: profile.words,
              })}
            </p>
          </div>
        )}
        {status && (
          <div className="flex flex-wrap items-center gap-x-3 gap-y-2.5 px-4 py-3">
            {ready ? (
              <p className="min-w-0 flex-1 text-sm text-success">
                {profile ? t("voice.updateReady") : null}
              </p>
            ) : (
              <>
                <progress
                  className="h-1.5 w-full basis-full"
                  max={status.nextAt}
                  value={status.totalWords}
                  aria-label={
                    profile
                      ? t("voice.nextUpdate", { count: remaining })
                      : t("voice.generatesIn", { count: remaining })
                  }
                />
                <p className="min-w-0 flex-1 text-sm text-muted-foreground tabular-nums">
                  {profile
                    ? t("voice.nextUpdate", { count: remaining })
                    : t("voice.generatesIn", { count: remaining })}
                </p>
              </>
            )}
            <Button
              size="sm"
              variant={ready ? "default" : "outline"}
              disabled={!ready || building}
              onClick={generate}
            >
              {profile ? t("voice.regenerate") : t("voice.generate")}
            </Button>
          </div>
        )}
      </Group>
      {building && <Note role="status">{t("voice.building")}</Note>}
      {problem?.kind === "notEnoughStored" && (
        <div
          className="flex gap-2.5 rounded-lg border bg-muted/50 px-3.5 py-2.5 text-sm"
          role="alert"
        >
          <Info aria-hidden="true" className="mt-px size-4 shrink-0 text-muted-foreground" />
          <div className="flex min-w-0 flex-col gap-0.5">
            <p className="font-medium">{t("voice.notEnoughTitle")}</p>
            <p>{t("voice.notEnoughStored", { stored: problem.stored })}</p>
            <p>{t("voice.notEnoughMore", { more: problem.needed - problem.stored })}</p>
          </div>
        </div>
      )}
      {(problem?.kind === "failed" || problem?.kind === "notYet") && (
        <p className="px-1 text-sm text-destructive" role="alert">
          {t("voice.failed")}
          {problem.kind === "failed" && problem.message && ` (${problem.message})`}
        </p>
      )}
      <Note>{t("voice.privacy", { provider: PROVIDER_NAMES[provider] })}</Note>
    </Section>
  );
}
