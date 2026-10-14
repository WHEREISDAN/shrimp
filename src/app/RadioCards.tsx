import { useId, type KeyboardEvent } from "react";
import { CircleCheck } from "lucide-react";
import { cn } from "@/lib/utils";

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
    <div
      className={cn(
        "grid gap-2",
        layout === "grid" && "grid-cols-[repeat(auto-fit,minmax(8.5rem,1fr))]",
      )}
      role="radiogroup"
      aria-label={label}
    >
      {options.map((option, index) => {
        const id = `${baseId}-${index}`;
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            className="group/card relative flex flex-col gap-1 rounded-xl border bg-card p-3.5 text-left shadow-xs transition-[background-color,border-color,box-shadow] duration-150 hover:bg-muted/40 aria-checked:border-brand aria-checked:ring-1 aria-checked:ring-brand dark:hover:bg-white/[0.03]"
            aria-checked={option.value === value}
            aria-labelledby={`${id}-title`}
            aria-describedby={`${id}-description`}
            tabIndex={option.value === selected ? 0 : -1}
            onClick={() => onChange(option.value)}
            onKeyDown={(event) => onKeyDown(event, index)}
          >
            <span className="pr-6 font-medium" id={`${id}-title`}>
              {option.title}
            </span>
            <CircleCheck
              aria-hidden="true"
              className="absolute top-[15px] right-3.5 size-4 scale-75 text-brand opacity-0 transition-[opacity,scale] duration-200 ease-spring group-aria-checked/card:scale-100 group-aria-checked/card:opacity-100"
            />
            <span className="flex flex-1 flex-col gap-2.5 empty:hidden" id={`${id}-description`}>
              {option.description && (
                <span className="text-sm text-muted-foreground">{option.description}</span>
              )}
              {option.example && (
                <span className="mt-auto rounded-lg bg-muted px-2.5 py-1.5 text-sm">
                  {option.example}
                </span>
              )}
            </span>
          </button>
        );
      })}
    </div>
  );
}
