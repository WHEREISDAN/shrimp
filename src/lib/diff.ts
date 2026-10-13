/** A word-level diff between two texts, for the transform changes view. */

export interface DiffPart {
  kind: "same" | "added" | "removed";
  text: string;
}

/** Above this many token pairs, the diff shows the old and new text whole. */
const MAX_CELLS = 1_500_000;

function tokens(text: string): string[] {
  return text.split(/(\s+)/).filter((token) => token.length > 0);
}

function push(parts: DiffPart[], kind: DiffPart["kind"], text: string) {
  const last = parts[parts.length - 1];
  if (last && last.kind === kind) {
    last.text += text;
  } else {
    parts.push({ kind, text });
  }
}

export function diffWords(before: string, after: string): DiffPart[] {
  const a = tokens(before);
  const b = tokens(after);
  if (a.length * b.length > MAX_CELLS) {
    return [
      { kind: "removed", text: before },
      { kind: "added", text: after },
    ];
  }
  // lengths[i][j]: the longest common subsequence of a[i..] and b[j..].
  const lengths = Array.from({ length: a.length + 1 }, () =>
    new Array<number>(b.length + 1).fill(0),
  );
  for (let i = a.length - 1; i >= 0; i--) {
    const row = lengths[i] as number[];
    const below = lengths[i + 1] as number[];
    for (let j = b.length - 1; j >= 0; j--) {
      row[j] = a[i] === b[j] ? (below[j + 1] ?? 0) + 1 : Math.max(below[j] ?? 0, row[j + 1] ?? 0);
    }
  }
  const parts: DiffPart[] = [];
  let i = 0;
  let j = 0;
  while (i < a.length && j < b.length) {
    if (a[i] === b[j]) {
      push(parts, "same", a[i] as string);
      i++;
      j++;
    } else if ((lengths[i + 1]?.[j] ?? 0) >= (lengths[i]?.[j + 1] ?? 0)) {
      push(parts, "removed", a[i] as string);
      i++;
    } else {
      push(parts, "added", b[j] as string);
      j++;
    }
  }
  a.slice(i).forEach((token) => push(parts, "removed", token));
  b.slice(j).forEach((token) => push(parts, "added", token));
  return parts;
}
