import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
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
    <section className="card stack" aria-labelledby="voice-heading">
      <h2 id="voice-heading" ref={headingRef} tabIndex={-1}>
        {t("voice.title")}
      </h2>
      {!profile && <p className="muted">{t("voice.intro")}</p>}
      {profile && (
        <dl className="voice-profile">
          <div className="voice-archetype">
            <dt>{t("voice.archetype")}</dt>
            <dd>{profile.archetype}</dd>
          </div>
          {profile.description && <p className="voice-description">{profile.description}</p>}
          <div>
            <dt>{t("voice.catchphrase")}</dt>
            <dd>“{profile.catchphrase}”</dd>
          </div>
          <div>
            <dt>{t("voice.mostCorrected")}</dt>
            <dd>{profile.mostCorrected}</dd>
          </div>
          <div>
            <dt>{t("voice.activeTimes")}</dt>
            <dd>{profile.activeTimes}</dd>
          </div>
          <p className="muted voice-meta">
            {t("voice.generatedOn", {
              date: new Intl.DateTimeFormat(i18n.language, { dateStyle: "medium" }).format(
                new Date(profile.generatedAt),
              ),
              words: profile.words,
            })}
          </p>
        </dl>
      )}
      {status && (
        <div className="voice-progress">
          {ready ? (
            <p className="ok">{profile ? t("voice.updateReady") : null}</p>
          ) : (
            <>
              <progress
                max={status.nextAt}
                value={status.totalWords}
                aria-label={
                  profile
                    ? t("voice.nextUpdate", { count: remaining })
                    : t("voice.generatesIn", { count: remaining })
                }
              />
              <p className="muted">
                {profile
                  ? t("voice.nextUpdate", { count: remaining })
                  : t("voice.generatesIn", { count: remaining })}
              </p>
            </>
          )}
          <button
            type="button"
            className={ready ? "primary" : undefined}
            disabled={!ready || building}
            onClick={generate}
          >
            {profile ? t("voice.regenerate") : t("voice.generate")}
          </button>
        </div>
      )}
      {building && (
        <p className="muted" role="status">
          {t("voice.building")}
        </p>
      )}
      {problem?.kind === "notEnoughStored" && (
        <div className="banner" role="alert">
          <p className="card-title">{t("voice.notEnoughTitle")}</p>
          <p>{t("voice.notEnoughStored", { stored: problem.stored })}</p>
          <p>{t("voice.notEnoughMore", { more: problem.needed - problem.stored })}</p>
        </div>
      )}
      {(problem?.kind === "failed" || problem?.kind === "notYet") && (
        <p className="error" role="alert">
          {t("voice.failed")}
          {problem.kind === "failed" && problem.message && ` (${problem.message})`}
        </p>
      )}
      <p className="muted">{t("voice.privacy", { provider: PROVIDER_NAMES[provider] })}</p>
    </section>
  );
}
