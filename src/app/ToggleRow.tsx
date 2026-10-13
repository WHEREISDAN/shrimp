import { useId } from "react";

interface ToggleRowProps {
  label: string;
  description?: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}

/** A setting that is on or off, with its explanation. */
export function ToggleRow({ label, description, checked, disabled, onChange }: ToggleRowProps) {
  const id = useId();
  return (
    <div className="toggle-row">
      <div className="toggle-text">
        <label htmlFor={id} className="card-title">
          {label}
        </label>
        {description && (
          <p id={`${id}-description`} className="muted">
            {description}
          </p>
        )}
      </div>
      <input
        id={id}
        type="checkbox"
        role="switch"
        aria-checked={checked}
        checked={checked}
        disabled={disabled}
        aria-describedby={description ? `${id}-description` : undefined}
        onChange={(event) => onChange(event.target.checked)}
      />
    </div>
  );
}
