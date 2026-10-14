import { useEffect, useId, useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import { Row, RowText } from "@/components/settings";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { NativeSelect } from "@/components/ui/native-select";
import { listOllamaModels, type Settings, type SettingsPatch } from "../lib/ipc";

interface OllamaFieldsProps {
  ai: Settings["ai"];
  update: (patch: SettingsPatch) => Promise<void>;
}

/** Ollama's address and model: rows of the AI provider's group. */
export function OllamaFields({ ai, update }: OllamaFieldsProps) {
  const { t } = useTranslation();
  const addressId = useId();
  const modelId = useId();
  const [listing, setListing] = useState<{ key: string; models: string[] | null } | null>(null);
  const [attempt, setAttempt] = useState(0);
  const key = `${ai.ollamaUrl}#${attempt}`;

  useEffect(() => {
    let cancelled = false;
    listOllamaModels()
      .then((models) => !cancelled && setListing({ key, models }))
      .catch(() => !cancelled && setListing({ key, models: null }));
    return () => {
      cancelled = true;
    };
  }, [key]);

  const current = listing?.key === key ? listing : null;
  const models = current?.models ?? [];
  const failed = current !== null && current.models === null;

  const saveAddress = (address: string) => {
    if (address.trim() !== ai.ollamaUrl) {
      void update({ ai: { ollamaUrl: address.trim() } });
    }
  };
  const onAddressKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      event.currentTarget.blur();
    }
  };

  return (
    <>
      <Row>
        <RowText title={t("ollama.address")} htmlFor={addressId} />
        <Input
          id={addressId}
          key={ai.ollamaUrl}
          type="text"
          className="w-64 max-w-full"
          defaultValue={ai.ollamaUrl}
          autoComplete="off"
          spellCheck={false}
          onBlur={(event) => saveAddress(event.target.value)}
          onKeyDown={onAddressKeyDown}
        />
      </Row>
      <Row>
        <RowText title={t("ollama.model")} htmlFor={modelId} />
        <NativeSelect
          id={modelId}
          aria-label={t("ollama.model")}
          wrapperClassName="min-w-40 max-w-full"
          value={ai.ollamaModel}
          onChange={(event) => void update({ ai: { ollamaModel: event.target.value } })}
        >
          <option value="">{t("ollama.automatic")}</option>
          {ai.ollamaModel !== "" && !models.includes(ai.ollamaModel) && (
            <option value={ai.ollamaModel}>{ai.ollamaModel}</option>
          )}
          {models.map((model) => (
            <option key={model} value={model}>
              {model}
            </option>
          ))}
        </NativeSelect>
      </Row>
      {failed && (
        <Row>
          <p className="min-w-0 flex-1 basis-56 text-sm text-destructive">
            {t("ollama.notRunning", { url: ai.ollamaUrl })}
          </p>
          <Button size="sm" onClick={() => setAttempt((count) => count + 1)}>
            {t("ollama.retry")}
          </Button>
        </Row>
      )}
    </>
  );
}
