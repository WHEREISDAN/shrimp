import { Plus } from "lucide-react";
import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { Group, Row, RowText, Section } from "@/components/settings";
import { Button } from "@/components/ui/button";
import { NativeSelect } from "@/components/ui/native-select";
import type { CustomTransform, Settings } from "../lib/ipc";
import { CustomTransformCard } from "./CustomTransformCard";
import { Page } from "./Page";
import { PolishCard } from "./PolishCard";
import { PromptEngineerCard } from "./PromptEngineerCard";
import { ShortcutRecorder } from "./ShortcutRecorder";
import { ToggleRow } from "./ToggleRow";
import { useAutosave } from "./useAutosave";
import { useSettings } from "./useSettings";

const MAX_CUSTOM = 8;

/** The first custom transform suggests Opt+3 when nothing uses it; the backend stores it with the other shortcuts. */
function newTransform(existing: CustomTransform[], settings: Settings): CustomTransform {
  const taken = Object.values(settings.shortcuts.bindings).flat();
  return {
    id: crypto.randomUUID(),
    name: "",
    prompt: "",
    shortcut: existing.length === 0 && !taken.includes("Opt+3") ? "Opt+3" : null,
    samples: [],
  };
}

export function TransformsPage() {
  const { t } = useTranslation();
  const { settings, error, update } = useSettings();
  const autoApplyId = useId();
  // Custom transforms being edited, until the saved list catches up (TRF-03).
  const [edits, setEdits] = useState<CustomTransform[] | null>(null);

  useAutosave(edits, edits !== null, (current) => {
    if (!current || !settings) {
      return;
    }
    void update({ transforms: { custom: current } }).then(() =>
      setEdits((now) => (now === current ? null : now)),
    );
  });

  const drafts = edits ?? settings?.transforms.custom;
  if (!settings || !drafts) {
    return (
      <Page title={t("transforms.title")}>
        {error ? (
          <p className="text-sm text-destructive">{error}</p>
        ) : (
          <p className="text-muted-foreground">{t("common.loading")}</p>
        )}
      </Page>
    );
  }
  const transforms = settings.transforms;

  return (
    <Page title={t("transforms.title")}>
      {error && (
        <p className="text-sm text-destructive" role="alert">
          {error}
        </p>
      )}
      <Section headingId="transforms-heading" title={t("transforms.heading")}>
        <Group>
          <ToggleRow
            label={t("transforms.toggle")}
            description={t("transforms.toggleBody")}
            checked={transforms.enabled}
            onChange={(enabled) => void update({ transforms: { enabled } })}
          />
        </Group>
      </Section>
      <fieldset
        className="flex min-w-0 flex-col gap-7 transition-opacity duration-150 disabled:opacity-50"
        disabled={!transforms.enabled}
      >
        <PolishCard settings={settings} update={update} />
        <PromptEngineerCard settings={settings} update={update} />
        <div className="flex flex-col gap-3">
          {drafts.map((transform, index) => (
            <CustomTransformCard
              key={transform.id}
              index={index}
              transform={transform}
              onChange={(next) =>
                setEdits(drafts.map((other) => (other.id === next.id ? next : other)))
              }
              onDelete={() => {
                const next = drafts.filter((other) => other.id !== transform.id);
                setEdits(null);
                void update({
                  transforms: {
                    custom: next,
                    autoApply: transforms.autoApply === transform.id ? null : transforms.autoApply,
                  },
                });
              }}
            />
          ))}
          {drafts.length < MAX_CUSTOM && (
            <Button
              className="self-start"
              onClick={() => setEdits([...drafts, newTransform(drafts, settings)])}
            >
              <Plus aria-hidden="true" />
              {t("transforms.newTransform")}
            </Button>
          )}
        </div>
        <Section headingId="auto-apply-heading" title={t("transforms.afterDictation")}>
          <Group>
            <Row>
              <RowText
                title={t("transforms.autoApply")}
                description={t("transforms.autoApplyBody")}
                htmlFor={autoApplyId}
              />
              <NativeSelect
                id={autoApplyId}
                wrapperClassName="min-w-40"
                value={transforms.autoApply ?? ""}
                onChange={(event) =>
                  void update({ transforms: { autoApply: event.target.value || null } })
                }
              >
                <option value="">{t("transforms.off")}</option>
                <option value="polish">{t("transforms.polish.title")}</option>
                <option value="promptEngineer">{t("transforms.promptEngineer.title")}</option>
                {transforms.custom
                  .filter((transform) => transform.name.trim() && transform.prompt.trim())
                  .map((transform) => (
                    <option key={transform.id} value={transform.id}>
                      {transform.name}
                    </option>
                  ))}
              </NativeSelect>
            </Row>
            <div className="flex min-h-(--row-min-h) flex-col justify-center px-4 py-2.5">
              <ShortcutRecorder action="viewChanges" label={t("transforms.seeChanges")} />
            </div>
          </Group>
        </Section>
      </fieldset>
    </Page>
  );
}
