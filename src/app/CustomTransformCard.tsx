import { Plus } from "lucide-react";
import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
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
  const ids = useId();
  const name: string =
    transform.name || t("transforms.custom.fallbackName", { position: index + 1 });
  const setSample = (at: number, text: string) =>
    onChange({ ...transform, samples: transform.samples.map((s, i) => (i === at ? text : s)) });

  return (
    <section
      className="flex flex-col divide-y divide-separator rounded-xl border bg-card shadow-xs"
      aria-label={name}
    >
      <div className="flex flex-wrap items-end gap-x-3 gap-y-2 px-4 pt-3 pb-3.5">
        <div className="flex min-w-0 flex-1 basis-48 flex-col gap-1.5">
          <Label htmlFor={`${ids}-name`}>{t("transforms.custom.name")}</Label>
          <Input
            id={`${ids}-name`}
            type="text"
            maxLength={40}
            value={transform.name}
            onChange={(event) => onChange({ ...transform, name: event.target.value })}
          />
        </div>
        <Button variant="destructive" onClick={onDelete}>
          {t("common.delete")}
        </Button>
      </div>
      <div className="flex min-h-(--row-min-h) flex-col justify-center px-4 py-2.5">
        <ShortcutRecorder action={{ customTransform: index }} label={name} />
      </div>
      <div className="flex flex-col gap-1.5 px-4 pt-2.5 pb-3.5">
        <Label htmlFor={`${ids}-prompt`}>{t("transforms.custom.prompt")}</Label>
        <Textarea
          id={`${ids}-prompt`}
          rows={3}
          placeholder={t("transforms.custom.placeholder")}
          value={transform.prompt}
          onChange={(event) => onChange({ ...transform, prompt: event.target.value })}
        />
        {!transform.name.trim() || !transform.prompt.trim() ? (
          <p className="text-sm text-muted-foreground">{t("transforms.custom.incomplete")}</p>
        ) : null}
      </div>
      <div className="flex flex-col gap-3 px-4 pt-2.5 pb-3.5">
        <div className="flex flex-col gap-0.5">
          <h3 className="font-medium">{t("transforms.custom.samples")}</h3>
          <p className="text-sm text-muted-foreground">{t("transforms.custom.samplesBody")}</p>
        </div>
        {transform.samples.map((sample, at) => {
          const count = words(sample);
          const fits = count >= 50 && count <= 500;
          return (
            <div key={at} className="flex flex-col gap-1">
              <Textarea
                rows={3}
                aria-label={t("transforms.custom.sample", { position: at + 1 })}
                value={sample}
                onChange={(event) => setSample(at, event.target.value)}
              />
              <div className="flex items-center justify-between gap-3">
                <span
                  className={cn(
                    "px-0.5 text-xs tabular-nums",
                    fits ? "text-muted-foreground" : "text-destructive",
                  )}
                >
                  {t("transforms.custom.words", { count })}
                </span>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() =>
                    onChange({
                      ...transform,
                      samples: transform.samples.filter((_, i) => i !== at),
                    })
                  }
                >
                  {t("transforms.custom.removeSample")}
                </Button>
              </div>
            </div>
          );
        })}
        {transform.samples.length < MAX_SAMPLES && (
          <Button
            size="sm"
            className="self-start"
            onClick={() => onChange({ ...transform, samples: [...transform.samples, ""] })}
          >
            <Plus aria-hidden="true" />
            {t("transforms.custom.addSample")}
          </Button>
        )}
      </div>
    </section>
  );
}
