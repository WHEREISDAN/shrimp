import { useState } from "react";
import type { Settings, SettingsPatch } from "../lib/ipc";
import { ShortcutRecorder } from "./ShortcutRecorder";
import { useAutosave } from "./useAutosave";

interface PromptEngineerCardProps {
  settings: Settings;
  update: (patch: SettingsPatch) => Promise<void>;
}

export function PromptEngineerCard({ settings, update }: PromptEngineerCardProps) {
  const saved = settings.transforms.promptEngineer;
  // The text being edited, until the saved prompt catches up.
  const [draft, setDraft] = useState<string | null>(null);
  const prompt = draft ?? saved.prompt;
  useAutosave(prompt, draft !== null && draft.trim() !== "" && draft !== saved.prompt, (text) => {
    void update({ transforms: { promptEngineer: { prompt: text } } }).then(() =>
      setDraft((current) => (current === text ? null : current)),
    );
  });

  return (
    <section className="card stack" aria-labelledby="prompt-engineer-heading">
      <div className="row">
        <h2 id="prompt-engineer-heading">Prompt Engineer</h2>
        <button
          type="button"
          onClick={() => {
            setDraft(null);
            void update({ transforms: { promptEngineer: null } });
          }}
        >
          Reset
        </button>
      </div>
      <p className="muted">Turns rough thoughts into a structured prompt for an AI assistant.</p>
      <ShortcutRecorder action="promptEngineer" label="Prompt Engineer" />
      <label className="field">
        <span>Prompt</span>
        <textarea rows={4} value={prompt} onChange={(event) => setDraft(event.target.value)} />
      </label>
    </section>
  );
}
