import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Group, Note, Row, RowText, Section } from "@/components/settings";
import { NativeSelect } from "@/components/ui/native-select";
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
  const { t } = useTranslation();
  const providerId = useId();
  const provider =
    options.providers.find((candidate) => candidate.id === settings.ai.provider) ??
    options.providers[0];
  if (!provider) {
    return null;
  }
  const field = modelField(provider.id);

  return (
    <Section headingId="ai-heading" title={t("aiProvider.title")}>
      <PrivacyModeNote settings={settings} />
      <Group>
        <Row>
          <RowText title={t("aiProvider.provider")} htmlFor={providerId} />
          <NativeSelect
            id={providerId}
            aria-label={t("aiProvider.provider")}
            wrapperClassName="min-w-40 max-w-full"
            value={provider.id}
            onChange={(event) => {
              const next = options.providers.find(
                (candidate) => candidate.id === event.target.value,
              );
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
          </NativeSelect>
        </Row>
        {field && (
          <ModelSelect
            label={t("aiProvider.model")}
            models={provider.models}
            value={settings.ai[field]}
            onChange={(model) => void update({ ai: { [field]: model } })}
          />
        )}
        {/* Ollama runs without a key, so its fields never share the page with the key. */}
        {provider.id === "ollama" && <OllamaFields ai={settings.ai} update={update} />}
      </Group>
      {provider.key !== null && (
        <KeyRow
          account={provider.key}
          label={provider.label}
          saved={keys[provider.key]}
          onChange={onKeys}
        />
      )}
      <AiTestRow key={provider.id} />
      <Note>
        {provider.id === "ollama"
          ? t("aiProvider.ollamaNote")
          : t("aiProvider.cloudNote", { provider: provider.label })}
      </Note>
    </Section>
  );
}
