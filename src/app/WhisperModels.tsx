import type { TFunction } from "i18next";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  deleteModel,
  downloadModel,
  errorMessage,
  EVENTS,
  getModels,
  type ModelStatus,
} from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";

const megabytes = (bytes: number) => Math.round(bytes / 1_000_000);

interface WhisperModelsProps {
  /** The model chosen in the settings. */
  chosen: string;
  onChoose: (id: string) => void;
}

function stateText(model: ModelStatus, chosen: boolean, t: TFunction): string {
  switch (model.state) {
    case "missing":
      return chosen ? t("whisperModels.waiting") : t("whisperModels.notDownloaded");
    case "downloading":
      return t("whisperModels.downloading", {
        percent: Math.floor((model.received / Math.max(model.total, 1)) * 100),
      });
    case "downloaded":
      return chosen ? t("common.loading") : t("whisperModels.downloaded");
    case "active":
      return t("whisperModels.inUse");
    case "failed":
      return model.message;
  }
}

/** The Whisper models on this Mac (ENG-08): size, downloaded state, delete. A new choice is used from the next dictation. */
export function WhisperModels({ chosen, onChoose }: WhisperModelsProps) {
  const { t } = useTranslation();
  const [models, setModels] = useState<ModelStatus[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    getModels()
      .then((list) => !cancelled && setModels(list))
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);
  useTauriEvent<ModelStatus[]>(EVENTS.models, setModels);

  const run = (work: Promise<void>) => {
    setError(null);
    work.catch((err: unknown) => setError(errorMessage(err)));
  };

  return (
    <fieldset className="plain-fieldset stack">
      <legend className="card-title">{t("whisperModels.title")}</legend>
      <ul className="list" aria-label={t("whisperModels.list")}>
        {models.map((model) => {
          const isChosen = model.id === chosen;
          const downloaded = model.state === "downloaded" || model.state === "active";
          return (
            <li key={model.id} className="list-row">
              <label className="list-main check">
                <input
                  type="radio"
                  name="whisper-model"
                  checked={isChosen}
                  onChange={() => onChoose(model.id)}
                />
                <span>
                  <span className="list-title">{model.label}</span>
                  <span className="list-meta">
                    {t("whisperModels.size", { size: megabytes(model.sizeBytes) })}
                    {model.isDefault && ` · ${t("whisperModels.recommended")}`} ·{" "}
                    {stateText(model, isChosen, t)}
                  </span>
                </span>
              </label>
              <div className="row-actions">
                {model.state === "missing" && !isChosen && (
                  <button type="button" onClick={() => run(downloadModel(model.id))}>
                    {t("whisperModels.download")}
                  </button>
                )}
                {downloaded && !isChosen && !model.inUse && (
                  <button
                    type="button"
                    aria-label={t("whisperModels.deleteModel", { model: model.label })}
                    onClick={() => run(deleteModel(model.id))}
                  >
                    {t("common.delete")}
                  </button>
                )}
              </div>
            </li>
          );
        })}
      </ul>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <p className="muted">{t("whisperModels.note")}</p>
    </fieldset>
  );
}
