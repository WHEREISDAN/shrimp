import { useEffect, useState } from "react";
import { EVENTS, getModels, retryModel, type ModelStatus } from "../lib/ipc";
import { useTauriEvent } from "../lib/useTauriEvent";

const megabytes = (bytes: number) => Math.round(bytes / 1_000_000);

export function ModelCard() {
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
  return (
    <section className="card" aria-labelledby="model-heading">
      <h2 id="model-heading">Speech model</h2>
      {model ? (
        <>
          <p className="card-title">
            {model.label}{" "}
            <span className="muted">· {megabytes(model.sizeBytes)} MB · on device</span>
          </p>
          <ModelProgress model={model} />
        </>
      ) : (
        <p className="muted">Loading…</p>
      )}
    </section>
  );
}

function ModelProgress({ model }: { model: ModelStatus }) {
  switch (model.state) {
    case "missing":
      return <p className="muted">Waiting to download…</p>;
    case "downloading": {
      const percent = Math.floor((model.received / Math.max(model.total, 1)) * 100);
      return (
        <div className="progress-row">
          <progress max={100} value={percent} aria-label="Model download progress" />
          <span data-testid="model-progress">
            Downloading {percent}% ({megabytes(model.received)} of {megabytes(model.total)} MB)
          </span>
        </div>
      );
    }
    case "downloaded":
      return <p className="muted">Downloaded. Loading…</p>;
    case "active":
      return <p className="ok">Ready. Dictation runs entirely on this Mac.</p>;
    case "failed":
      return (
        <div className="progress-row">
          <span className="error">{model.message}</span>
          <button type="button" onClick={() => void retryModel().catch(() => undefined)}>
            Retry
          </button>
        </div>
      );
  }
}
