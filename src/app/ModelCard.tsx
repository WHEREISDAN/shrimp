import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { EVENTS, getModels, retryModel, type ModelStatus } from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";

const megabytes = (bytes: number) => Math.round(bytes / 1_000_000);

interface ModelCardProps {
  /** Home shows the card only while the model isn't ready. */
  hideWhenReady?: boolean;
}

export function ModelCard({ hideWhenReady = false }: ModelCardProps) {
  const { t } = useTranslation();
  const [models, setModels] = useState<ModelStatus[]>([]);

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

  // The model in use, or the one getting ready.
  const model =
    models.find((candidate) => candidate.inUse) ??
    models.find((candidate) => candidate.state === "downloading" || candidate.state === "failed") ??
    models.find((candidate) => candidate.isDefault);
  if (hideWhenReady && model?.state === "active") {
    return null;
  }
  return (
    <section className="card" aria-labelledby="model-heading">
      <h2 id="model-heading">{t("modelCard.title")}</h2>
      {model ? (
        <>
          <p className="card-title">
            {model.label}{" "}
            <span className="muted">
              · {t("modelCard.onDevice", { size: megabytes(model.sizeBytes) })}
            </span>
          </p>
          <ModelProgress model={model} />
        </>
      ) : (
        <p className="muted">{t("common.loading")}</p>
      )}
    </section>
  );
}

function ModelProgress({ model }: { model: ModelStatus }) {
  const { t } = useTranslation();
  switch (model.state) {
    case "missing":
      return <p className="muted">{t("modelCard.waiting")}</p>;
    case "downloading": {
      const percent = Math.floor((model.received / Math.max(model.total, 1)) * 100);
      return (
        <div className="progress-row">
          <progress max={100} value={percent} aria-label={t("modelCard.progress")} />
          <span data-testid="model-progress">
            {t("modelCard.downloading", {
              percent,
              received: megabytes(model.received),
              total: megabytes(model.total),
            })}
          </span>
        </div>
      );
    }
    case "downloaded":
      return <p className="muted">{t("modelCard.loading")}</p>;
    case "active":
      return <p className="ok">{t("modelCard.ready")}</p>;
    case "failed":
      return (
        <div className="progress-row">
          <span className="error">{model.message}</span>
          <button type="button" onClick={() => void retryModel().catch(() => undefined)}>
            {t("modelCard.retry")}
          </button>
        </div>
      );
  }
}
