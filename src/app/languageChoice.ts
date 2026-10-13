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

/** The names of `ids`, in their order. */
export function languageNames(ids: string[], options: LanguageOption[]): string[] {
  return ids.map((id) => options.find((option) => option.id === id)?.name ?? id);
}
