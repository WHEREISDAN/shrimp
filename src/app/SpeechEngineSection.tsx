import type { AiOptions, ApiKeys, Settings, SettingsPatch, SttEngineId } from "../lib/ipc";
import { KeyRow } from "./KeyRow";
import { ModelSelect } from "./ModelSelect";
import { RadioCards, type RadioCardOption } from "./RadioCards";
import { WhisperModels } from "./WhisperModels";

function modelField(id: SttEngineId): "openaiModel" | "groqModel" | null {
  switch (id) {
    case "openai":
      return "openaiModel";
    case "groq":
      return "groqModel";
    case "local-whisper":
      return null;
  }
}

interface SpeechEngineSectionProps {
  settings: Settings;
  options: AiOptions;
  keys: ApiKeys;
  onKeys: (keys: ApiKeys) => void;
  update: (patch: SettingsPatch) => Promise<void>;
}

export function SpeechEngineSection({
  settings,
  options,
  keys,
  onKeys,
  update,
}: SpeechEngineSectionProps) {
  const engine = options.sttEngines.find((candidate) => candidate.id === settings.stt.engine);
  const field = engine ? modelField(engine.id) : null;
  const choices: RadioCardOption<SttEngineId>[] = options.sttEngines.map((candidate) => ({
    value: candidate.id,
    title: candidate.label,
  }));
  // Keys are shared with the AI provider of the same company, so name them that way.
  const company =
    options.providers.find((provider) => provider.key === engine?.key)?.label ?? engine?.label;

  return (
    <section className="card stack" aria-labelledby="speech-heading">
      <h2 id="speech-heading">Speech engine</h2>
      <RadioCards
        label="Speech engine"
        value={settings.stt.engine}
        options={choices}
        onChange={(id) => void update({ stt: { engine: id } })}
      />
      {settings.stt.engine === "local-whisper" && (
        <WhisperModels
          chosen={settings.stt.localModel}
          onChoose={(id) => void update({ stt: { localModel: id } })}
        />
      )}
      {engine && field && (
        <ModelSelect
          label="Speech model"
          models={engine.models}
          value={settings.stt[field]}
          onChange={(model) => void update({ stt: { [field]: model } })}
        />
      )}
      {engine?.key != null && company && (
        <KeyRow account={engine.key} label={company} saved={keys[engine.key]} onChange={onKeys} />
      )}
      <p className="muted">
        Whisper on this Mac is private and works offline. Cloud engines send your audio to OpenAI or
        Groq; if one fails, Shrimp falls back to Whisper on this Mac.
      </p>
    </section>
  );
}
