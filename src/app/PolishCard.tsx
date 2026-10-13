import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { PolishRules, Settings, SettingsPatch } from "../lib/ipc";
import { ShortcutRecorder } from "./ShortcutRecorder";
import { ToggleRow } from "./ToggleRow";

/** The rules in display order; each has `transforms.polish.<rule>` and `<rule>Body` text. */
const RULES: (keyof PolishRules)[] = ["concise", "clarity", "reorder", "structure", "tone"];

const MAX_INSTRUCTIONS = 5;
const MAX_INSTRUCTION_WORDS = 50;
const words = (text: string) => text.trim().split(/\s+/).filter(Boolean).length;

interface PolishCardProps {
  settings: Settings;
  update: (patch: SettingsPatch) => Promise<void>;
}

export function PolishCard({ settings, update }: PolishCardProps) {
  const { t } = useTranslation();
  const polish = settings.transforms.polish;
  const [draft, setDraft] = useState("");
  const [problem, setProblem] = useState<string | null>(null);

  const add = () => {
    const text = draft.trim();
    if (!text) {
      return;
    }
    if (words(text) > MAX_INSTRUCTION_WORDS) {
      setProblem(t("transforms.polish.tooLong", { count: MAX_INSTRUCTION_WORDS }));
      return;
    }
    if (polish.instructions.some((other) => other.toLowerCase() === text.toLowerCase())) {
      setProblem(t("transforms.polish.duplicate"));
      return;
    }
    setProblem(null);
    setDraft("");
    void update({ transforms: { polish: { instructions: [...polish.instructions, text] } } });
  };

  return (
    <section className="card stack" aria-labelledby="polish-heading">
      <div className="row">
        <h2 id="polish-heading">{t("transforms.polish.title")}</h2>
        <button
          type="button"
          onClick={() => void update({ transforms: { polish: { rules: null } } })}
        >
          {t("transforms.reset")}
        </button>
      </div>
      <p className="muted">{t("transforms.polish.body")}</p>
      <ShortcutRecorder action="polish" label={t("transforms.polish.title")} />
      {RULES.map((rule) => (
        <ToggleRow
          key={rule}
          label={t(`transforms.polish.${rule}`)}
          description={t(`transforms.polish.${rule}Body`)}
          checked={polish.rules[rule]}
          onChange={(on) => void update({ transforms: { polish: { rules: { [rule]: on } } } })}
        />
      ))}
      <h3 className="subheading">{t("transforms.polish.instructions")}</h3>
      <ul className="rows">
        {polish.instructions.map((instruction, index) => (
          <li key={instruction} className="row">
            <span>{instruction}</span>
            <button
              type="button"
              aria-label={t("transforms.polish.removeInstruction", { instruction })}
              onClick={() =>
                void update({
                  transforms: {
                    polish: { instructions: polish.instructions.filter((_, i) => i !== index) },
                  },
                })
              }
            >
              {t("transforms.polish.remove")}
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
            aria-label={t("transforms.polish.newInstruction")}
            placeholder={t("transforms.polish.placeholder")}
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
          />
          <button type="submit" disabled={!draft.trim()}>
            {t("transforms.polish.add")}
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
