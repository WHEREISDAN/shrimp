import { useId, useState, type KeyboardEvent } from "react";
import type { AppCategory, Style } from "../lib/ipc";
import { RadioCards, type RadioCardOption } from "./RadioCards";
import { CATEGORIES, STYLE_TEXT } from "./styleCatalog";

const LAST_TAB = CATEGORIES.length - 1;

interface StylesSectionProps {
  styles: Record<AppCategory, Style>;
  onChange: (category: AppCategory, style: Style) => void;
}

export function StylesSection({ styles, onChange }: StylesSectionProps) {
  const [index, setIndex] = useState(0);
  const baseId = useId();
  const category = CATEGORIES[index] ?? CATEGORIES[0];
  if (!category) {
    return null;
  }

  const options: RadioCardOption<Style>[] = Object.entries(category.examples).map(
    ([style, example]) => ({ value: style as Style, ...STYLE_TEXT[style as Style], example }),
  );

  const select = (next: number, focus: boolean) => {
    setIndex(next);
    if (focus) {
      document.getElementById(`${baseId}-tab-${next}`)?.focus();
    }
  };
  const onKeyDown = (event: KeyboardEvent) => {
    const targets: Record<string, number> = {
      ArrowRight: index === LAST_TAB ? 0 : index + 1,
      ArrowLeft: index === 0 ? LAST_TAB : index - 1,
      Home: 0,
      End: LAST_TAB,
    };
    const target = targets[event.key];
    if (target !== undefined) {
      event.preventDefault();
      select(target, true);
    }
  };

  return (
    <section className="card stack" aria-labelledby="styles-heading">
      <h2 id="styles-heading">Styles</h2>
      <div className="tabs" role="tablist" aria-label="Styles">
        {CATEGORIES.map((item, position) => (
          <button
            key={item.id}
            id={`${baseId}-tab-${position}`}
            type="button"
            role="tab"
            className="tab"
            aria-selected={position === index}
            aria-controls={position === index ? `${baseId}-panel` : undefined}
            tabIndex={position === index ? 0 : -1}
            onClick={() => select(position, false)}
            onKeyDown={onKeyDown}
          >
            {item.label}
          </button>
        ))}
      </div>
      <div
        id={`${baseId}-panel`}
        role="tabpanel"
        aria-labelledby={`${baseId}-tab-${index}`}
        className="tab-panel"
      >
        <p className="muted style-apps">
          {category.appList && <span className="style-apps-label">Applies to </span>}
          {category.apps}
        </p>
        <RadioCards
          label={`${category.label} style`}
          value={styles[category.id]}
          options={options}
          layout="grid"
          onChange={(style) => onChange(category.id, style)}
        />
        {category.id !== "other" && (
          <button type="button" className="next" onClick={() => select(index + 1, false)}>
            Next
          </button>
        )}
      </div>
      <p className="muted footnote">
        Styles apply to new dictations. AI assistants such as ChatGPT and Claude get no style
        formatting.
      </p>
    </section>
  );
}
