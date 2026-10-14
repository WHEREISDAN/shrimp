import type { TFunction } from "i18next";
import { useEffect, useState } from "react";
import { Download, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Note } from "@/components/settings";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
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

const percentOf = (received: number, total: number) =>
  Math.floor((received / Math.max(total, 1)) * 100);

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
        percent: percentOf(model.received, model.total),
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
    <fieldset className="mt-2 min-w-0">
      <legend className="px-1 pb-2 font-medium">{t("whisperModels.title")}</legend>
      <ul
        className="list flex flex-col divide-y divide-separator rounded-xl border bg-card shadow-xs"
        aria-label={t("whisperModels.list")}
      >
        {models.map((model) => {
          const isChosen = model.id === chosen;
          const downloaded = model.state === "downloaded" || model.state === "active";
          return (
            <li
              key={model.id}
              className="list-row flex min-h-(--row-min-h) flex-wrap items-center gap-x-3 gap-y-2 px-4 py-2.5"
            >
              <label className="flex min-w-0 flex-1 basis-56 items-start gap-3">
                <input
                  type="radio"
                  name="whisper-model"
                  className="mt-0.5"
                  checked={isChosen}
                  onChange={() => onChoose(model.id)}
                />
                <span className="flex min-w-0 flex-col gap-0.5">
                  <span className="list-title">{model.label}</span>
                  <span
                    className={cn(
                      "text-sm tabular-nums",
                      model.state === "failed" ? "text-destructive" : "text-muted-foreground",
                    )}
                  >
                    {t("whisperModels.size", { size: megabytes(model.sizeBytes) })}
                    {model.isDefault && ` · ${t("whisperModels.recommended")}`} ·{" "}
                    {stateText(model, isChosen, t)}
                  </span>
                </span>
              </label>
              <div className="flex items-center gap-1 empty:hidden">
                {model.state === "missing" && !isChosen && (
                  <Button size="sm" onClick={() => run(downloadModel(model.id))}>
                    <Download aria-hidden="true" />
                    {t("whisperModels.download")}
                  </Button>
                )}
                {downloaded && !isChosen && !model.inUse && (
                  <Button
                    variant="ghost"
                    size="sm"
                    aria-label={t("whisperModels.deleteModel", { model: model.label })}
                    onClick={() => run(deleteModel(model.id))}
                  >
                    <Trash2 aria-hidden="true" />
                    {t("common.delete")}
                  </Button>
                )}
              </div>
              {model.state === "downloading" && (
                <div className="basis-full pb-0.5 pl-7">
                  <progress
                    className="h-1.5 w-full"
                    max={100}
                    value={percentOf(model.received, model.total)}
                    aria-label={t("modelCard.progress")}
                  />
                </div>
              )}
            </li>
          );
        })}
      </ul>
      {error && (
        <p className="mt-2 px-1 text-sm text-destructive" role="alert">
          {error}
        </p>
      )}
      <Note className="mt-2">{t("whisperModels.note")}</Note>
    </fieldset>
  );
}
