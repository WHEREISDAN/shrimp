import { useState } from "react";
import { useTranslation } from "react-i18next";
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
        {error ? <p className="error">{error}</p> : <p className="muted">{t("common.loading")}</p>}
      </Page>
    );
  }
  const transforms = settings.transforms;

  return (
    <Page title={t("transforms.title")}>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <section className="card stack" aria-labelledby="transforms-heading">
        <h2 id="transforms-heading">{t("transforms.heading")}</h2>
        <ToggleRow
          label={t("transforms.toggle")}
          description={t("transforms.toggleBody")}
          checked={transforms.enabled}
          onChange={(enabled) => void update({ transforms: { enabled } })}
        />
      </section>
      <fieldset className="plain-fieldset stack" disabled={!transforms.enabled}>
        <PolishCard settings={settings} update={update} />
        <PromptEngineerCard settings={settings} update={update} />
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
          <button
            type="button"
            className="add-transform"
            onClick={() => setEdits([...drafts, newTransform(drafts, settings)])}
          >
            {t("transforms.newTransform")}
          </button>
        )}
        <section className="card stack" aria-labelledby="auto-apply-heading">
          <h2 id="auto-apply-heading">{t("transforms.afterDictation")}</h2>
          <label className="select-field field">
            <span>{t("transforms.autoApply")}</span>
            <select
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
            </select>
          </label>
          <p className="muted">{t("transforms.autoApplyBody")}</p>
          <ShortcutRecorder action="viewChanges" label={t("transforms.seeChanges")} />
        </section>
      </fieldset>
    </Page>
  );
}
