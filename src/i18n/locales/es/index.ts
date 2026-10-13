import type { Catalog } from "../en";
import { controls } from "./controls";
import { core } from "./core";
import { library } from "./library";
import { settingsSections } from "./settingsSections";
import { styleTransforms } from "./styleTransforms";

/** The interface in Spanish (LANG-06). */
export const es: Catalog = {
  ...core,
  ...library,
  ...styleTransforms,
  ...settingsSections,
  ...controls,
};
