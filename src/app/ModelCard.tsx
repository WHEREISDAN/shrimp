import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Group, Row, Section } from "@/components/settings";
import { Button } from "@/components/ui/button";
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
    <Section headingId="model-heading" title={t("modelCard.title")}>
      <Group>
        <Row className="flex-col items-stretch gap-2 py-3.5">
          {model ? (
            <>
              <p className="font-medium">
                {model.label}{" "}
                <span className="font-normal text-muted-foreground">
                  · {t("modelCard.onDevice", { size: megabytes(model.sizeBytes) })}
                </span>
              </p>
              <ModelProgress model={model} />
            </>
          ) : (
            <p className="text-muted-foreground">{t("common.loading")}</p>
          )}
        </Row>
      </Group>
    </Section>
  );
}

function ModelProgress({ model }: { model: ModelStatus }) {
  const { t } = useTranslation();
  switch (model.state) {
    case "missing":
      return <p className="text-sm text-muted-foreground">{t("modelCard.waiting")}</p>;
    case "downloading": {
      const percent = Math.floor((model.received / Math.max(model.total, 1)) * 100);
      return (
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
          <progress
            className="min-w-32 flex-1"
            max={100}
            value={percent}
            aria-label={t("modelCard.progress")}
          />
          <span className="text-sm text-muted-foreground tabular-nums" data-testid="model-progress">
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
      return <p className="text-sm text-muted-foreground">{t("modelCard.loading")}</p>;
    case "active":
      return <p className="text-sm text-success">{t("modelCard.ready")}</p>;
    case "failed":
      return (
        <div className="flex flex-wrap items-center justify-between gap-3">
          <span className="text-sm text-destructive">{model.message}</span>
          <Button size="sm" onClick={() => void retryModel().catch(() => undefined)}>
            {t("modelCard.retry")}
          </Button>
        </div>
      );
  }
}
