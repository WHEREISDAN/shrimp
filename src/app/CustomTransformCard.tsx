import { setShortcut, type CustomTransform, type Settings } from "../lib/ipc";
import { ShortcutRecorder } from "./ShortcutRecorder";

const MAX_SAMPLES = 5;
const words = (text: string) => text.trim().split(/\s+/).filter(Boolean).length;

interface CustomTransformCardProps {
  index: number;
  transform: CustomTransform;
  onChange: (transform: CustomTransform) => void;
  onDelete: () => void;
  apply: (settings: Settings) => void;
}

/** One custom transform; edits save on their own (TRF-03). */
export function CustomTransformCard({
  index,
  transform,
  onChange,
  onDelete,
  apply,
}: CustomTransformCardProps) {
  const name = transform.name || `Transform ${index + 1}`;
  const setSample = (at: number, text: string) =>
    onChange({ ...transform, samples: transform.samples.map((s, i) => (i === at ? text : s)) });

  return (
    <section className="card stack" aria-label={name}>
      <div className="row">
        <label className="field grow">
          <span>Name</span>
          <input
            type="text"
            maxLength={40}
            value={transform.name}
            onChange={(event) => onChange({ ...transform, name: event.target.value })}
          />
        </label>
        <button type="button" className="danger" onClick={onDelete}>
          Delete
        </button>
      </div>
      <ShortcutRecorder
        label={name}
        value={transform.shortcut}
        onSave={async (shortcut) => apply(await setShortcut({ customTransform: index }, shortcut))}
      />
      <label className="field">
        <span>Prompt</span>
        <textarea
          rows={3}
          placeholder="For example: rewrite this as a friendly reply to a customer"
          value={transform.prompt}
          onChange={(event) => onChange({ ...transform, prompt: event.target.value })}
        />
      </label>
      {!transform.name.trim() || !transform.prompt.trim() ? (
        <p className="muted">Give it a name and a prompt to use it.</p>
      ) : null}
      <h3 className="subheading">Writing samples</h3>
      <p className="muted">Up to 5 samples of 50 to 500 words whose style the result follows.</p>
      {transform.samples.map((sample, at) => {
        const count = words(sample);
        const fits = count >= 50 && count <= 500;
        return (
          <div key={at} className="field">
            <textarea
              rows={3}
              aria-label={`Sample ${at + 1}`}
              value={sample}
              onChange={(event) => setSample(at, event.target.value)}
            />
            <div className="row">
              <span className={fits ? "muted" : "error"}>{`${count} words`}</span>
              <button
                type="button"
                onClick={() =>
                  onChange({ ...transform, samples: transform.samples.filter((_, i) => i !== at) })
                }
              >
                Remove sample
              </button>
            </div>
          </div>
        );
      })}
      {transform.samples.length < MAX_SAMPLES && (
        <button
          type="button"
          onClick={() => onChange({ ...transform, samples: [...transform.samples, ""] })}
        >
          Add sample
        </button>
      )}
    </section>
  );
}
