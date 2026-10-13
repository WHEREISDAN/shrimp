import { useTranslation } from "react-i18next";
import type { CustomTransform } from "../lib/ipc";
import { ShortcutRecorder } from "./ShortcutRecorder";

const MAX_SAMPLES = 5;
const words = (text: string) => text.trim().split(/\s+/).filter(Boolean).length;

interface CustomTransformCardProps {
  index: number;
  transform: CustomTransform;
  onChange: (transform: CustomTransform) => void;
  onDelete: () => void;
}

/** One custom transform; edits save on their own (TRF-03). */
export function CustomTransformCard({
  index,
  transform,
  onChange,
  onDelete,
}: CustomTransformCardProps) {
  const { t } = useTranslation();
  const name: string =
    transform.name || t("transforms.custom.fallbackName", { position: index + 1 });
  const setSample = (at: number, text: string) =>
    onChange({ ...transform, samples: transform.samples.map((s, i) => (i === at ? text : s)) });

  return (
    <section className="card stack" aria-label={name}>
      <div className="row">
        <label className="field grow">
          <span>{t("transforms.custom.name")}</span>
          <input
            type="text"
            maxLength={40}
            value={transform.name}
            onChange={(event) => onChange({ ...transform, name: event.target.value })}
          />
        </label>
        <button type="button" className="danger" onClick={onDelete}>
          {t("common.delete")}
        </button>
      </div>
      <ShortcutRecorder action={{ customTransform: index }} label={name} />
      <label className="field">
        <span>{t("transforms.custom.prompt")}</span>
        <textarea
          rows={3}
          placeholder={t("transforms.custom.placeholder")}
          value={transform.prompt}
          onChange={(event) => onChange({ ...transform, prompt: event.target.value })}
        />
      </label>
      {!transform.name.trim() || !transform.prompt.trim() ? (
        <p className="muted">{t("transforms.custom.incomplete")}</p>
      ) : null}
      <h3 className="subheading">{t("transforms.custom.samples")}</h3>
      <p className="muted">{t("transforms.custom.samplesBody")}</p>
      {transform.samples.map((sample, at) => {
        const count = words(sample);
        const fits = count >= 50 && count <= 500;
        return (
          <div key={at} className="field">
            <textarea
              rows={3}
              aria-label={t("transforms.custom.sample", { position: at + 1 })}
              value={sample}
              onChange={(event) => setSample(at, event.target.value)}
            />
            <div className="row">
              <span className={fits ? "muted" : "error"}>
                {t("transforms.custom.words", { count })}
              </span>
              <button
                type="button"
                onClick={() =>
                  onChange({ ...transform, samples: transform.samples.filter((_, i) => i !== at) })
                }
              >
                {t("transforms.custom.removeSample")}
              </button>
            </div>
          </div>
        );
      })}
      {transform.samples.length < MAX_SAMPLES && (
        <button
          type="button"
          onClick={() => onChange({ ...transform, samples: [...transform.samples, ""] })}
        >
          {t("transforms.custom.addSample")}
        </button>
      )}
    </section>
  );
}
