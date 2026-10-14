import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Group, Note, Section } from "@/components/settings";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { PolishRules, Settings, SettingsPatch } from "../lib/ipc";
import { ShortcutRecorder } from "./ShortcutRecorder";
import { ToggleRow } from "./ToggleRow";

/** The rules in display order; each has `transforms.polish.<rule>` and `<rule>Body` text. */
const RULES: (keyof PolishRules)[] = [
  "concise",
  "clarity",
  "reorder",
  "structure",
  "tone",
  "mainPoint",
  "impact",
];

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
    if (polish.instructions.some((other) => other.trim().toLowerCase() === text.toLowerCase())) {
      setProblem(t("transforms.polish.duplicate"));
      return;
    }
    setProblem(null);
    setDraft("");
    void update({ transforms: { polish: { instructions: [...polish.instructions, text] } } });
  };

  return (
    <Section
      headingId="polish-heading"
      title={t("transforms.polish.title")}
      action={
        <Button size="sm" onClick={() => void update({ transforms: { polish: { rules: null } } })}>
          {t("transforms.reset")}
        </Button>
      }
    >
      <Note>{t("transforms.polish.body")}</Note>
      <Group>
        <div className="flex min-h-(--row-min-h) flex-col justify-center px-4 py-2.5">
          <ShortcutRecorder action="polish" label={t("transforms.polish.title")} />
        </div>
        {RULES.map((rule) => (
          <ToggleRow
            key={rule}
            label={t(`transforms.polish.${rule}`)}
            description={t(`transforms.polish.${rule}Body`)}
            checked={polish.rules[rule]}
            onChange={(on) => void update({ transforms: { polish: { rules: { [rule]: on } } } })}
          />
        ))}
      </Group>
      <h3 className="mt-3 px-1 text-sm font-medium text-muted-foreground">
        {t("transforms.polish.instructions")}
      </h3>
      <Group>
        <ul className="flex flex-col divide-y divide-separator empty:hidden">
          {polish.instructions.map((instruction, index) => (
            <li
              key={instruction}
              className="flex min-h-(--row-min-h) items-center justify-between gap-3 px-4 py-2.5"
            >
              <span className="min-w-0 [overflow-wrap:anywhere]">{instruction}</span>
              <Button
                variant="ghost"
                size="sm"
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
              </Button>
            </li>
          ))}
        </ul>
        {polish.instructions.length < MAX_INSTRUCTIONS && (
          <form
            className="flex items-center gap-2 px-4 py-2.5"
            onSubmit={(event) => {
              event.preventDefault();
              add();
            }}
          >
            <Input
              type="text"
              className="flex-1"
              aria-label={t("transforms.polish.newInstruction")}
              placeholder={t("transforms.polish.placeholder")}
              value={draft}
              onChange={(event) => setDraft(event.target.value)}
            />
            <Button type="submit" disabled={!draft.trim()}>
              {t("transforms.polish.add")}
            </Button>
          </form>
        )}
      </Group>
      {problem && (
        <p className="px-1 text-sm text-destructive" role="alert">
          {problem}
        </p>
      )}
    </Section>
  );
}
