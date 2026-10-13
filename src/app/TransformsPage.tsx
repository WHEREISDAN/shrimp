import { useState } from "react";
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
      <Page title="Transforms">
        {error ? <p className="error">{error}</p> : <p className="muted">Loading…</p>}
      </Page>
    );
  }
  const transforms = settings.transforms;

  return (
    <Page title="Transforms">
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <section className="card stack" aria-labelledby="transforms-heading">
        <h2 id="transforms-heading">Rewrite any text with AI</h2>
        <ToggleRow
          label="Transforms"
          description="Select text anywhere and press a transform's shortcut: the result replaces it, and “See changes” shows what changed. Your text is sent to your AI provider."
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
            onChange={(next) => setEdits(drafts.map((t) => (t.id === next.id ? next : t)))}
            onDelete={() => {
              const next = drafts.filter((t) => t.id !== transform.id);
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
            New transform
          </button>
        )}
        <section className="card stack" aria-labelledby="auto-apply-heading">
          <h2 id="auto-apply-heading">After dictation</h2>
          <label className="select-field field">
            <span>Apply a transform after every dictation</span>
            <select
              value={transforms.autoApply ?? ""}
              onChange={(event) =>
                void update({ transforms: { autoApply: event.target.value || null } })
              }
            >
              <option value="">Off</option>
              <option value="polish">Polish</option>
              <option value="promptEngineer">Prompt Engineer</option>
              {transforms.custom
                .filter((transform) => transform.name.trim() && transform.prompt.trim())
                .map((transform) => (
                  <option key={transform.id} value={transform.id}>
                    {transform.name}
                  </option>
                ))}
            </select>
          </label>
          <p className="muted">If the transform fails, your dictation is pasted as it was.</p>
          <ShortcutRecorder action="viewChanges" label="See changes" />
        </section>
      </fieldset>
    </Page>
  );
}
