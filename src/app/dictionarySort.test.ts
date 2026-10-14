import { describe, expect, it } from "vitest";
import { sortEntries } from "./dictionarySort";
import { entry } from "./testBackend";

const ENTRIES = [
  entry({ id: 1, term: "zeta", uses: 1, createdAt: 300 }),
  entry({ id: 2, term: "Alpha", uses: 5, createdAt: 100 }),
  entry({ id: 3, term: "Kubernetes", misspelling: "cooper netties", createdAt: 200 }),
  entry({ id: 4, term: "beta", pinned: true, uses: 0, createdAt: 50 }),
  entry({ id: 5, term: "Ünal", uses: 1, createdAt: 300 }),
];

const terms = (sort: Parameters<typeof sortEntries>[1]) =>
  sortEntries(ENTRIES, sort, "en").map((item) => item.term);

describe("sortEntries", () => {
  it("puts starred words first, then the most used, then the newest", () => {
    expect(terms("starred")).toEqual(["beta", "Alpha", "Ünal", "zeta", "Kubernetes"]);
  });

  it("sorts by date either way, the latest added first for equal dates", () => {
    expect(terms("newest")).toEqual(["Ünal", "zeta", "Kubernetes", "Alpha", "beta"]);
    expect(terms("oldest")).toEqual(["beta", "Alpha", "Kubernetes", "zeta", "Ünal"]);
  });

  it("sorts A to Z by what the row shows first, ignoring case and accents", () => {
    expect(terms("alphabetical")).toEqual(["Alpha", "beta", "Kubernetes", "Ünal", "zeta"]);
  });

  it("sorts by use alone for Most used", () => {
    expect(terms("mostUsed")).toEqual(["Alpha", "Ünal", "zeta", "Kubernetes", "beta"]);
  });

  it("leaves the list it was given alone", () => {
    const before = ENTRIES.map((item) => item.id);
    sortEntries(ENTRIES, "alphabetical", "en");
    expect(ENTRIES.map((item) => item.id)).toEqual(before);
  });
});
