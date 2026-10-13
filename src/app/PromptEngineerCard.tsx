import { useState } from "react";
import { setShortcut, type Settings, type SettingsPatch } from "../lib/ipc";
import { ShortcutRecorder } from "./ShortcutRecorder";
import { useAutosave } from "./useAutosave";

interface PromptEngineerCardProps {
  settings: Settings;
  update: (patch: SettingsPatch) => Promise<void>;
  apply: (settings: Settings) => void;
}

export function PromptEngineerCard({ settings, update, apply }: PromptEngineerCardProps) {
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
      <ShortcutRecorder
        label="Prompt Engineer"
        value={saved.shortcut}
        onSave={async (shortcut) => apply(await setShortcut("promptEngineer", shortcut))}
      />
      <label className="field">
        <span>Prompt</span>
        <textarea rows={4} value={prompt} onChange={(event) => setDraft(event.target.value)} />
      </label>
    </section>
  );
}
