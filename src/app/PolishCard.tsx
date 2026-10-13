import { useState } from "react";
import { setShortcut, type PolishRules, type Settings, type SettingsPatch } from "../lib/ipc";
import { ShortcutRecorder } from "./ShortcutRecorder";
import { ToggleRow } from "./ToggleRow";

const RULES: { key: keyof PolishRules; label: string; description: string }[] = [
  {
    key: "concise",
    label: "Make more concise",
    description: "Cuts filler, repetition and padding.",
  },
  {
    key: "clarity",
    label: "Reword for clarity",
    description: "Replaces vague or awkward phrasing.",
  },
  {
    key: "reorder",
    label: "Reorder for readability",
    description: "Puts the main point first and related points together.",
  },
  {
    key: "structure",
    label: "Add structure for readability",
    description: "Short paragraphs, and lists for enumerated items.",
  },
  { key: "tone", label: "Maintain your tone", description: "Keeps your voice as it is." },
];

const MAX_INSTRUCTIONS = 5;
const MAX_INSTRUCTION_WORDS = 50;
const words = (text: string) => text.trim().split(/\s+/).filter(Boolean).length;

interface PolishCardProps {
  settings: Settings;
  update: (patch: SettingsPatch) => Promise<void>;
  apply: (settings: Settings) => void;
}

export function PolishCard({ settings, update, apply }: PolishCardProps) {
  const polish = settings.transforms.polish;
  const [draft, setDraft] = useState("");
  const [problem, setProblem] = useState<string | null>(null);

  const add = () => {
    const text = draft.trim();
    if (!text) {
      return;
    }
    if (words(text) > MAX_INSTRUCTION_WORDS) {
      setProblem(`Keep each instruction under ${MAX_INSTRUCTION_WORDS} words.`);
      return;
    }
    if (polish.instructions.some((other) => other.toLowerCase() === text.toLowerCase())) {
      setProblem("That instruction is already in the list.");
      return;
    }
    setProblem(null);
    setDraft("");
    void update({ transforms: { polish: { instructions: [...polish.instructions, text] } } });
  };

  return (
    <section className="card stack" aria-labelledby="polish-heading">
      <div className="row">
        <h2 id="polish-heading">Polish</h2>
        <button
          type="button"
          onClick={() => void update({ transforms: { polish: { rules: null } } })}
        >
          Reset
        </button>
      </div>
      <p className="muted">
        Rewrites the selected text, or the text box, or your last dictation from the past minute.
      </p>
      <ShortcutRecorder
        label="Polish"
        value={polish.shortcut}
        onSave={async (shortcut) => apply(await setShortcut("polish", shortcut))}
      />
      {RULES.map((rule) => (
        <ToggleRow
          key={rule.key}
          label={rule.label}
          description={rule.description}
          checked={polish.rules[rule.key]}
          onChange={(on) => void update({ transforms: { polish: { rules: { [rule.key]: on } } } })}
        />
      ))}
      <h3 className="subheading">Custom instructions</h3>
      <ul className="rows">
        {polish.instructions.map((instruction, index) => (
          <li key={instruction} className="row">
            <span>{instruction}</span>
            <button
              type="button"
              aria-label={`Remove ${instruction}`}
              onClick={() =>
                void update({
                  transforms: {
                    polish: { instructions: polish.instructions.filter((_, i) => i !== index) },
                  },
                })
              }
            >
              Remove
            </button>
          </li>
        ))}
      </ul>
      {polish.instructions.length < MAX_INSTRUCTIONS && (
        <form
          className="row"
          onSubmit={(event) => {
            event.preventDefault();
            add();
          }}
        >
          <input
            type="text"
            aria-label="New instruction"
            placeholder="For example: use British spelling"
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
          />
          <button type="submit" disabled={!draft.trim()}>
            Add
          </button>
        </form>
      )}
      {problem && (
        <p className="error" role="alert">
          {problem}
        </p>
      )}
    </section>
  );
}
