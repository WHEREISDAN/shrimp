import { createInstance } from "i18next";
import { useEffect } from "react";
import { initReactI18next } from "react-i18next";
import { IS_WINDOWS } from "../lib/platform";
import { de } from "./locales/de";
import { windows as deWindows } from "./locales/de/windows";
import { en, type Catalog, type WindowsCatalog } from "./locales/en";
import { windows as enWindows } from "./locales/en/windows";
import { es } from "./locales/es";
import { windows as esWindows } from "./locales/es/windows";
import { it } from "./locales/it";
import { windows as itWindows } from "./locales/it/windows";
import { pt } from "./locales/pt";
import { windows as ptWindows } from "./locales/pt/windows";

/** The languages the interface is translated into (LANG-06). */
export const UI_LANGUAGES = ["en", "de", "es", "it", "pt"] as const;
export type UiLanguage = (typeof UI_LANGUAGES)[number];

export const CATALOGS: Record<UiLanguage, Catalog> = { en, de, es, it, pt };

/** Texts that replace the macOS wording on Windows, per language. */
export const WINDOWS_TEXT: Record<UiLanguage, WindowsCatalog> = {
  en: enWindows,
  de: deWindows,
  es: esWindows,
  it: itWindows,
  pt: ptWindows,
};

type Tree = { [key: string]: string | Tree };

/** `base` with every text of `overrides` put in its place. */
export function overlay<T extends object>(base: T, overrides: object): T {
  const merged: Tree = { ...(base as Tree) };
  for (const [key, value] of Object.entries(overrides as Tree)) {
    const current = merged[key];
    merged[key] =
      typeof value === "string" || typeof current !== "object" ? value : overlay(current, value);
  }
  return merged as T;
}

/** The catalog for `language` on this platform. */
export function catalogFor(language: UiLanguage, windows = IS_WINDOWS): Catalog {
  return windows ? overlay(CATALOGS[language], WINDOWS_TEXT[language]) : CATALOGS[language];
}

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
      UI_LANGUAGES.map((code) => [code, { translation: catalogFor(code) }]),
    ),
    lng: language,
    fallbackLng: "en",
    interpolation: { escapeValue: false },
    // Resources are bundled, so i18next is ready before the first render.
    initAsync: false,
  });
  return i18n;
}
