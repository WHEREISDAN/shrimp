import { createInstance } from "i18next";
import { useEffect } from "react";
import { initReactI18next } from "react-i18next";
import { de } from "./locales/de";
import { en, type Catalog } from "./locales/en";
import { es } from "./locales/es";
import { it } from "./locales/it";
import { pt } from "./locales/pt";

/** The languages the interface is translated into (LANG-06). */
export const UI_LANGUAGES = ["en", "de", "es", "it", "pt"] as const;
export type UiLanguage = (typeof UI_LANGUAGES)[number];

export const CATALOGS: Record<UiLanguage, Catalog> = { en, de, es, it, pt };

/** The first of `preferred` (BCP 47 tags) Shrimp is translated into, else English. */
export function resolveLanguage(preferred: readonly string[]): UiLanguage {
  for (const tag of preferred) {
    const base = tag.toLowerCase().split("-")[0];
    const match = UI_LANGUAGES.find((language) => language === base);
    if (match) {
      return match;
    }
  }
  return "en";
}

/** The interface language for Settings › General › App language. */
export function languageFor(setting: string): UiLanguage {
  return setting === "system" ? resolveLanguage(navigator.languages) : resolveLanguage([setting]);
}

/** Follows the App language setting, and tells screen readers the page's language. */
export function useInterfaceLanguage(setting: string | undefined) {
  useEffect(() => {
    if (!setting) {
      return;
    }
    const language = languageFor(setting);
    document.documentElement.lang = language;
    void i18n.changeLanguage(language);
  }, [setting]);
}

export const i18n = createInstance();

/** Starts i18next in `language`; English fills any gap. */
export function setupI18n(language: UiLanguage = resolveLanguage(navigator.languages)) {
  void i18n.use(initReactI18next).init({
    resources: Object.fromEntries(
      Object.entries(CATALOGS).map(([code, translation]) => [code, { translation }]),
    ),
    lng: language,
    fallbackLng: "en",
    interpolation: { escapeValue: false },
    // Resources are bundled, so i18next is ready before the first render.
    initAsync: false,
  });
  return i18n;
}
