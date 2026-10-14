import { createCn } from "cn/config";

/** Joins class names and resolves Tailwind conflicts; knows the theme's extra text size. */
export const cn = createCn({
  extend: { classGroups: { "font-size": [{ text: ["2xs"] }] } },
});
