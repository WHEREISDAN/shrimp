import { useEffect, useState, type KeyboardEvent } from "react";
import { listOllamaModels, type Settings, type SettingsPatch } from "../lib/ipc";

interface OllamaFieldsProps {
  ai: Settings["ai"];
  update: (patch: SettingsPatch) => Promise<void>;
}

export function OllamaFields({ ai, update }: OllamaFieldsProps) {
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
      <label className="select-field field">
        <span>Address</span>
        <input
          key={ai.ollamaUrl}
          type="text"
          defaultValue={ai.ollamaUrl}
          autoComplete="off"
          spellCheck={false}
          onBlur={(event) => saveAddress(event.target.value)}
          onKeyDown={onAddressKeyDown}
        />
      </label>
      <label className="select-field field">
        <span>Ollama model</span>
        <select
          aria-label="Ollama model"
          value={ai.ollamaModel}
          onChange={(event) => void update({ ai: { ollamaModel: event.target.value } })}
        >
          <option value="">Automatic (recommended)</option>
          {ai.ollamaModel !== "" && !models.includes(ai.ollamaModel) && (
            <option value={ai.ollamaModel}>{ai.ollamaModel}</option>
          )}
          {models.map((model) => (
            <option key={model} value={model}>
              {model}
            </option>
          ))}
        </select>
      </label>
      {failed && (
        <div className="inline-row">
          <p className="error">{`Ollama isn't running at ${ai.ollamaUrl}.`}</p>
          <button type="button" onClick={() => setAttempt((count) => count + 1)}>
            Retry
          </button>
        </div>
      )}
    </>
  );
}
