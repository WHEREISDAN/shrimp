import { controls } from "./controls";
import { core } from "./core";
import { library } from "./library";
import { settingsSections } from "./settingsSections";
import { styleTransforms } from "./styleTransforms";

/**
 * The English interface text, the source for every other language (LANG-06).
 * Each module holds one area's keys under top-level names no other module uses.
 */
export const en = {
  ...core,
  ...library,
  ...styleTransforms,
  ...settingsSections,
  ...controls,
} as const;

type Messages<T> = { [K in keyof T]: T[K] extends string ? string : Messages<T[K]> };

/** What every translation must provide: the same keys as English. */
export type Catalog = Messages<typeof en>;
export type CoreCatalog = Messages<typeof core>;
export type LibraryCatalog = Messages<typeof library>;
export type StyleTransformsCatalog = Messages<typeof styleTransforms>;
export type SettingsSectionsCatalog = Messages<typeof settingsSections>;
export type ControlsCatalog = Messages<typeof controls>;
