import { describe, expect, it } from "vitest";
import { CATALOGS, catalogFor, i18n, languageFor, resolveLanguage, WINDOWS_TEXT } from ".";

type Tree = { [key: string]: string | Tree };

function flatten(tree: Tree, prefix = ""): Map<string, string> {
  return new Map(
    Object.entries(tree).flatMap(([key, value]) =>
      typeof value === "string"
        ? [[`${prefix}${key}`, value] as const]
        : [...flatten(value, `${prefix}${key}.`)],
    ),
  );
}

/** `{{name}}` placeholders and `<tag>` markup, which a translation must keep. */
function markers(text: string): string[] {
  const placeholders = [...text.matchAll(/\{\{\s*(\w+)[^}]*\}\}/g)].map((match) => match[1] ?? "");
  const tags = [...text.matchAll(/<\/?(\w+)\s*\/?>/g)].map((match) => match[0]);
  return [...placeholders, ...tags].sort();
}

const english = flatten(CATALOGS.en as Tree);

describe.each(Object.entries(CATALOGS).filter(([language]) => language !== "en"))(
  "the %s interface",
  (_, catalog) => {
    const translated = flatten(catalog as Tree);

    it("has every English key and no other", () => {
      expect([...translated.keys()].sort()).toEqual([...english.keys()].sort());
    });

    it("keeps every placeholder and tag", () => {
      const changed = [...english].filter(
        ([key, text]) => markers(translated.get(key) ?? "").join() !== markers(text).join(),
      );
      expect(changed.map(([key]) => key)).toEqual([]);
    });

    it("translates the text", () => {
      const empty = [...translated].filter(([, text]) => text.trim() === "");
      expect(empty.map(([key]) => key)).toEqual([]);
      const same = [...translated].filter(([key, text]) => text === english.get(key));
      expect(same.length / english.size).toBeLessThan(0.15);
    });
  },
);

describe("the Windows texts", () => {
  const windowsEnglish = flatten(WINDOWS_TEXT.en as Tree);
  const base = flatten(CATALOGS.en as Tree);

  it("replace existing English texts", () => {
    expect([...windowsEnglish.keys()].filter((key) => !base.has(key))).toEqual([]);
  });

  it.each(Object.entries(WINDOWS_TEXT))("are complete in %s", (_, catalog) => {
    const texts = flatten(catalog as Tree);
    expect([...texts.keys()].sort()).toEqual([...windowsEnglish.keys()].sort());
    const changed = [...windowsEnglish].filter(
      ([key, text]) => markers(texts.get(key) ?? "").join() !== markers(text).join(),
    );
    expect(changed.map(([key]) => key)).toEqual([]);
    expect([...texts.values()].filter((text) => /\bMac\b|macOS|⌘|⌥/.test(text))).toEqual([]);
  });

  it("take the place of the macOS wording only on Windows", () => {
    expect(catalogFor("de", true).help.about).toBe(WINDOWS_TEXT.de.help.about);
    expect(catalogFor("de", true).help.keyBack).toBe(CATALOGS.de.help.keyBack);
    expect(catalogFor("de", false).help.about).toBe(CATALOGS.de.help.about);
  });
});

describe("the interface language", () => {
  it("follows the setting, else the Mac, else English", () => {
    expect(resolveLanguage(["fr-FR", "de-AT"])).toBe("de");
    expect(resolveLanguage(["pt-BR"])).toBe("pt");
    expect(resolveLanguage(["ja"])).toBe("en");
    expect(languageFor("it")).toBe("it");
  });

  it("switches the text", async () => {
    await i18n.changeLanguage("de");
    expect(i18n.t("common.cancel")).toBe(CATALOGS.de.common.cancel);
    expect(i18n.t("common.cancel")).not.toBe("Cancel");
    await i18n.changeLanguage("en");
    expect(i18n.t("common.cancel")).toBe("Cancel");
  });
});
