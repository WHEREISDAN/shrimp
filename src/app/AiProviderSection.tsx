import type { AiOptions, ApiKeys, LlmProviderId, Settings, SettingsPatch } from "../lib/ipc";
import { AiTestRow } from "./AiTestRow";
import { KeyRow } from "./KeyRow";
import { ModelSelect } from "./ModelSelect";
import { OllamaFields } from "./OllamaFields";
import { PrivacyModeNote } from "./PrivacyModeNote";

type ProviderModelField = "anthropicModel" | "openaiModel" | "groqModel";

function modelField(id: LlmProviderId): ProviderModelField | null {
  switch (id) {
    case "anthropic":
      return "anthropicModel";
    case "openai":
      return "openaiModel";
    case "groq":
      return "groqModel";
    case "ollama":
      return null;
  }
}

interface AiProviderSectionProps {
  settings: Settings;
  options: AiOptions;
  keys: ApiKeys;
  onKeys: (keys: ApiKeys) => void;
  update: (patch: SettingsPatch) => Promise<void>;
}

export function AiProviderSection({
  settings,
  options,
  keys,
  onKeys,
  update,
}: AiProviderSectionProps) {
  const provider =
    options.providers.find((candidate) => candidate.id === settings.ai.provider) ??
    options.providers[0];
  if (!provider) {
    return null;
  }
  const field = modelField(provider.id);

  return (
    <section className="card stack" aria-labelledby="ai-heading">
      <h2 id="ai-heading">AI provider</h2>
      <PrivacyModeNote settings={settings} />
      <label className="select-field field">
        <span>Provider</span>
        <select
          aria-label="Provider"
          value={provider.id}
          onChange={(event) => {
            const next = options.providers.find((candidate) => candidate.id === event.target.value);
            if (next) {
              void update({ ai: { provider: next.id } });
            }
          }}
        >
          {options.providers.map((candidate) => (
            <option key={candidate.id} value={candidate.id}>
              {candidate.label}
            </option>
          ))}
        </select>
      </label>
      {field && (
        <ModelSelect
          label="Model"
          models={provider.models}
          value={settings.ai[field]}
          onChange={(model) => void update({ ai: { [field]: model } })}
        />
      )}
      {provider.key !== null && (
        <KeyRow
          account={provider.key}
          label={provider.label}
          saved={keys[provider.key]}
          onChange={onKeys}
        />
      )}
      {provider.id === "ollama" && <OllamaFields ai={settings.ai} update={update} />}
      <AiTestRow key={provider.id} />
      <p className="muted">
        {provider.id === "ollama"
          ? "Ollama runs on this Mac; nothing leaves it."
          : `Your dictation text is sent to ${provider.label} for cleanup, with your own key.`}
      </p>
    </section>
  );
}
