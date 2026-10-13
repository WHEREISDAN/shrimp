import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  getApiKeys,
  type ApiKeys,
  type KeyAccount,
  type Settings,
  type SettingsPatch,
} from "../../lib/ipc";
import { KeyRow } from "../KeyRow";
import { ModelCard } from "../ModelCard";
import { RadioCards } from "../RadioCards";

type Choice = "none" | "ollama" | KeyAccount;

const COMPANIES: Record<KeyAccount, string> = {
  anthropic: "Anthropic",
  openai: "OpenAI",
  groq: "Groq",
};

interface EngineStepProps {
  settings: Settings;
  update: (patch: SettingsPatch) => Promise<void>;
}

/** The Whisper model downloading with progress, and an optional AI provider key (ONB-05). */
export function EngineStep({ settings, update }: EngineStepProps) {
  const { t } = useTranslation();
  const [keys, setKeys] = useState<ApiKeys | null>(null);
  useEffect(() => {
    getApiKeys()
      .then(setKeys)
      .catch(() => undefined);
  }, []);

  const choice: Choice = settings.ai.cleanup === "none" ? "none" : settings.ai.provider;
  const choose = (next: Choice) =>
    void update(
      next === "none"
        ? { ai: { cleanup: "none" } }
        : {
            ai: {
              provider: next,
              cleanup: settings.ai.cleanup === "none" ? "light" : settings.ai.cleanup,
            },
          },
    );

  return (
    <>
      <p>{t("onboarding.engineBody")}</p>
      <ModelCard />
      <h2 className="section-title">{t("onboarding.aiTitle")}</h2>
      <p className="muted">{t("onboarding.aiBody")}</p>
      <RadioCards
        label={t("onboarding.aiTitle")}
        value={choice}
        onChange={choose}
        options={[
          { value: "none", title: t("onboarding.aiNone"), description: t("onboarding.aiNoneBody") },
          { value: "ollama", title: "Ollama", description: t("onboarding.aiOllamaBody") },
          { value: "anthropic", title: "Anthropic", description: t("onboarding.aiCloudBody") },
          { value: "openai", title: "OpenAI", description: t("onboarding.aiCloudBody") },
          { value: "groq", title: "Groq", description: t("onboarding.aiCloudBody") },
        ]}
      />
      {keys && choice !== "none" && choice !== "ollama" && (
        <KeyRow
          account={choice}
          label={COMPANIES[choice]}
          saved={keys[choice]}
          onChange={setKeys}
        />
      )}
    </>
  );
}
