import type { DictionaryEntry, DictionarySort } from "../lib/ipc";

/** The Dictionary page's Sort by choices, in menu order (DIC-04). */
export const DICTIONARY_SORTS: DictionarySort[] = [
  "starred",
  "newest",
  "oldest",
  "alphabetical",
  "mostUsed",
];

type Compare = (a: DictionaryEntry, b: DictionaryEntry) => number;

const newest: Compare = (a, b) => b.createdAt - a.createdAt || b.id - a.id;
const mostUsed: Compare = (a, b) => b.uses - a.uses || newest(a, b);
const starred: Compare = (a, b) => Number(b.pinned) - Number(a.pinned) || mostUsed(a, b);
const oldest: Compare = (a, b) => -newest(a, b);

/** What a row shows first: the misspelling of a correction, else the word. */
const written = (entry: DictionaryEntry) => entry.misspelling ?? entry.term;

function alphabetical(locale: string): Compare {
  const collator = new Intl.Collator(locale, { sensitivity: "base", numeric: true });
  return (a, b) =>
    collator.compare(written(a), written(b)) || collator.compare(a.term, b.term) || a.id - b.id;
}

/** `entries` in the order `sort` asks for; Starred first is the list's own order. */
export function sortEntries(
  entries: readonly DictionaryEntry[],
  sort: DictionarySort,
  locale: string,
): DictionaryEntry[] {
  const compare: Record<DictionarySort, () => Compare> = {
    starred: () => starred,
    newest: () => newest,
    oldest: () => oldest,
    alphabetical: () => alphabetical(locale),
    mostUsed: () => mostUsed,
  };
  return [...entries].sort(compare[sort]());
}
