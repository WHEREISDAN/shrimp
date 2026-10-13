import type { TFunction } from "i18next";
import type { AppCategory, Style } from "../lib/ipc";

/** A style card's title and description, in the interface language. */
export const styleText = (t: TFunction, style: Style) => ({
  title: t(`style.options.${style}.title`),
  description: t(`style.options.${style}.description`),
});

interface CategoryInfo {
  id: AppCategory;
  label: string;
  /** Whether `apps` is a list of names rather than a sentence. */
  appList: boolean;
  apps: string;
  /** Same words in every style; only capitals and punctuation change. */
  examples: Partial<Record<Style, string>>;
}

/** The app categories in tab order, with their text in the interface language. */
export function styleCategories(t: TFunction): CategoryInfo[] {
  return [
    {
      id: "personal",
      label: t("style.categories.personal.label"),
      appList: true,
      apps: t("style.categories.personal.apps"),
      examples: {
        formal: t("style.categories.personal.formal"),
        casual: t("style.categories.personal.casual"),
        veryCasual: t("style.categories.personal.veryCasual"),
      },
    },
    {
      id: "work",
      label: t("style.categories.work.label"),
      appList: true,
      apps: t("style.categories.work.apps"),
      examples: {
        formal: t("style.categories.work.formal"),
        casual: t("style.categories.work.casual"),
        excited: t("style.categories.work.excited"),
      },
    },
    {
      id: "email",
      label: t("style.categories.email.label"),
      appList: true,
      apps: t("style.categories.email.apps"),
      examples: {
        formal: t("style.categories.email.formal"),
        casual: t("style.categories.email.casual"),
        excited: t("style.categories.email.excited"),
      },
    },
    {
      id: "other",
      label: t("style.categories.other.label"),
      appList: false,
      apps: t("style.categories.other.apps"),
      examples: {
        formal: t("style.categories.other.formal"),
        casual: t("style.categories.other.casual"),
        excited: t("style.categories.other.excited"),
      },
    },
  ];
}
