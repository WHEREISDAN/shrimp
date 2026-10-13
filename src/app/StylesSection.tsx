import { useId, useState, type KeyboardEvent } from "react";
import { Trans, useTranslation } from "react-i18next";
import type { AppCategory, Style } from "../lib/ipc";
import { RadioCards, type RadioCardOption } from "./RadioCards";
import { styleCategories, styleText } from "./styleCatalog";

interface StylesSectionProps {
  styles: Record<AppCategory, Style>;
  onChange: (category: AppCategory, style: Style) => void;
}

export function StylesSection({ styles, onChange }: StylesSectionProps) {
  const { t } = useTranslation();
  const [index, setIndex] = useState(0);
  const baseId = useId();
  const categories = styleCategories(t);
  const lastTab = categories.length - 1;
  const category = categories[index] ?? categories[0];
  if (!category) {
    return null;
  }

  const options: RadioCardOption<Style>[] = Object.entries(category.examples).map(
    ([style, example]) => ({ value: style as Style, ...styleText(t, style as Style), example }),
  );

  const select = (next: number, focus: boolean) => {
    setIndex(next);
    if (focus) {
      document.getElementById(`${baseId}-tab-${next}`)?.focus();
    }
  };
  const onKeyDown = (event: KeyboardEvent) => {
    const targets: Record<string, number> = {
      ArrowRight: index === lastTab ? 0 : index + 1,
      ArrowLeft: index === 0 ? lastTab : index - 1,
      Home: 0,
      End: lastTab,
    };
    const target = targets[event.key];
    if (target !== undefined) {
      event.preventDefault();
      select(target, true);
    }
  };

  return (
    <section className="card stack" aria-labelledby="styles-heading">
      <h2 id="styles-heading">{t("style.styles.title")}</h2>
      <div className="tabs" role="tablist" aria-label={t("style.styles.tabs")}>
        {categories.map((item, position) => (
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
          {category.appList ? (
            <Trans
              i18nKey="style.styles.appliesTo"
              values={{ apps: category.apps }}
              components={{ label: <span className="style-apps-label" /> }}
            />
          ) : (
            category.apps
          )}
        </p>
        <RadioCards
          label={t("style.styles.group", { category: category.label })}
          value={styles[category.id]}
          options={options}
          layout="grid"
          onChange={(style) => onChange(category.id, style)}
        />
        {category.id !== "other" && (
          <button type="button" className="next" onClick={() => select(index + 1, false)}>
            {t("style.styles.next")}
          </button>
        )}
      </div>
      <p className="muted footnote">{t("style.styles.footnote")}</p>
    </section>
  );
}
