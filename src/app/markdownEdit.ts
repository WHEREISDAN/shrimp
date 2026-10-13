/** A text value with the selection that should follow an edit. */
export interface TextEdit {
  value: string;
  start: number;
  end: number;
}

const LINK_PLACEHOLDER = "link text";
const LINK_URL = "https://";

/** Wraps the selection in `mark`, or inserts an empty pair with the caret between. */
export function wrapSelection(value: string, start: number, end: number, mark: string): TextEdit {
  const selected = value.slice(start, end);
  return {
    value: `${value.slice(0, start)}${mark}${selected}${mark}${value.slice(end)}`,
    start: start + mark.length,
    end: end + mark.length,
  };
}

/** Turns the selection into `[text](https://)` and selects the URL for typing over. */
export function insertLink(value: string, start: number, end: number): TextEdit {
  const label = value.slice(start, end) || LINK_PLACEHOLDER;
  const urlStart = start + label.length + 3;
  return {
    value: `${value.slice(0, start)}[${label}](${LINK_URL})${value.slice(end)}`,
    start: urlStart,
    end: urlStart + LINK_URL.length,
  };
}

const MARKUP = /\\(.)|\[([^\]]*)\]\([^)]*\)|\*\*|\*|_/g;

/** One line of plain text for a list row: no emphasis marks, links reduced to their text. */
export function plainPreview(markdown: string): string {
  return markdown
    .replace(
      MARKUP,
      (_match, escaped?: string, linkText?: string) =>
        escaped ?? (linkText === undefined ? "" : plainPreview(linkText)),
    )
    .replace(/\s+/g, " ")
    .trim();
}
