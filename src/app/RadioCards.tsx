import { useId, type KeyboardEvent } from "react";

const ARROW_STEP: Record<string, number> = {
  ArrowRight: 1,
  ArrowDown: 1,
  ArrowLeft: -1,
  ArrowUp: -1,
};

export interface RadioCardOption<T extends string> {
  value: T;
  title: string;
  description?: string;
  example?: string;
}

interface RadioCardsProps<T extends string> {
  label: string;
  value: T;
  options: RadioCardOption<T>[];
  onChange: (value: T) => void;
  layout?: "stack" | "grid";
}

/** A radio group of cards. Arrow keys move the selection, as with native radios. */
export function RadioCards<T extends string>({
  label,
  value,
  options,
  onChange,
  layout = "stack",
}: RadioCardsProps<T>) {
  const baseId = useId();
  const selected = options.some((option) => option.value === value) ? value : options[0]?.value;

  const onKeyDown = (event: KeyboardEvent<HTMLElement>, index: number) => {
    const step = ARROW_STEP[event.key];
    if (step === undefined) {
      return;
    }
    event.preventDefault();
    const nextIndex = (index + step + options.length) % options.length;
    const next = options[nextIndex];
    if (next) {
      onChange(next.value);
      const radios =
        event.currentTarget.parentElement?.querySelectorAll<HTMLElement>("[role='radio']");
      radios?.[nextIndex]?.focus();
    }
  };

  return (
    <div className={`radio-cards ${layout}`} role="radiogroup" aria-label={label}>
      {options.map((option, index) => {
        const id = `${baseId}-${index}`;
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            className="radio-card"
            aria-checked={option.value === value}
            aria-labelledby={`${id}-title`}
            aria-describedby={`${id}-description`}
            tabIndex={option.value === selected ? 0 : -1}
            onClick={() => onChange(option.value)}
            onKeyDown={(event) => onKeyDown(event, index)}
          >
            <span className="radio-card-title" id={`${id}-title`}>
              {option.title}
            </span>
            <span className="radio-card-body" id={`${id}-description`}>
              {option.description && <span className="muted">{option.description}</span>}
              {option.example && <span className="radio-card-example">{option.example}</span>}
            </span>
          </button>
        );
      })}
    </div>
  );
}
