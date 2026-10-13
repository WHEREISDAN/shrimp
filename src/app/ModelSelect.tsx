import type { ModelOption } from "../lib/ipc";

interface ModelSelectProps {
  label: string;
  models: ModelOption[];
  value: string;
  onChange: (id: string) => void;
}

export function ModelSelect({ label, models, value, onChange }: ModelSelectProps) {
  const selected = models.find((model) => model.id === value);
  return (
    <div className="field">
      <label className="select-field">
        <span>{label}</span>
        <select aria-label={label} value={value} onChange={(event) => onChange(event.target.value)}>
          {!selected && value !== "" && <option value={value}>{value}</option>}
          {models.map((model) => (
            <option key={model.id} value={model.id}>
              {model.label}
            </option>
          ))}
        </select>
      </label>
      {selected?.note && <p className="muted hint">{selected.note}</p>}
    </div>
  );
}
