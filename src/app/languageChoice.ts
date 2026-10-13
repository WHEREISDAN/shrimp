import type { LanguageOption } from "../lib/ipc";

/** The selection with `id` turned on or off: a variant replaces the other variants of its language. */
export function toggleLanguage(
  selected: string[],
  id: string,
  options: LanguageOption[],
): string[] {
  if (selected.includes(id)) {
    return selected.filter((other) => other !== id);
  }
  const group = options.find((option) => option.id === id)?.group ?? null;
  const kept = group
    ? selected.filter((other) => options.find((option) => option.id === other)?.group !== group)
    : selected;
  return [...kept, id];
}

const displayNames = new Map<string, Intl.DisplayNames | null>();

function namesIn(language: string): Intl.DisplayNames | null {
  if (!displayNames.has(language)) {
    let names: Intl.DisplayNames | null = null;
    try {
      names = new Intl.DisplayNames([language], { type: "language", fallback: "none" });
    } catch {
      names = null;
    }
    displayNames.set(language, names);
  }
  return displayNames.get(language) ?? null;
}

/**
 * The name of language `id` in the interface `language`. English keeps the catalog's own names,
 * such as "English (UK)"; other languages ask the system, falling back to the catalog's name.
 */
export function languageName(id: string, options: LanguageOption[], language = "en"): string {
  const own = options.find((option) => option.id === id)?.name ?? id;
  if (language === "en" || language.startsWith("en-")) {
    return own;
  }
  let name: string | undefined;
  try {
    name = namesIn(language)?.of(id);
  } catch {
    name = undefined;
  }
  return name ? name.charAt(0).toLocaleUpperCase(language) + name.slice(1) : own;
}

/** The names of `ids`, in their order, in the interface `language`. */
export function languageNames(ids: string[], options: LanguageOption[], language = "en"): string[] {
  return ids.map((id) => languageName(id, options, language));
}
