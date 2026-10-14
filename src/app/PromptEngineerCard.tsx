import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { Group, Note, Section } from "@/components/settings";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import type { Settings, SettingsPatch } from "../lib/ipc";
import { ShortcutRecorder } from "./ShortcutRecorder";
import { useAutosave } from "./useAutosave";

interface PromptEngineerCardProps {
  settings: Settings;
  update: (patch: SettingsPatch) => Promise<void>;
}

export function PromptEngineerCard({ settings, update }: PromptEngineerCardProps) {
  const { t } = useTranslation();
  const promptId = useId();
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
    <Section
      headingId="prompt-engineer-heading"
      title={t("transforms.promptEngineer.title")}
      action={
        <Button
          size="sm"
          onClick={() => {
            setDraft(null);
            void update({ transforms: { promptEngineer: null } });
          }}
        >
          {t("transforms.reset")}
        </Button>
      }
    >
      <Note>{t("transforms.promptEngineer.body")}</Note>
      <Group>
        <div className="flex min-h-(--row-min-h) flex-col justify-center px-4 py-2.5">
          <ShortcutRecorder action="promptEngineer" label={t("transforms.promptEngineer.title")} />
        </div>
        <div className="flex flex-col gap-1.5 px-4 pt-2.5 pb-3.5">
          <Label htmlFor={promptId}>{t("transforms.promptEngineer.prompt")}</Label>
          <Textarea
            id={promptId}
            rows={4}
            value={prompt}
            onChange={(event) => setDraft(event.target.value)}
          />
        </div>
      </Group>
    </Section>
  );
}
