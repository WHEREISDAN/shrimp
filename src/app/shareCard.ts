import type { TFunction } from "i18next";
import { BRAND_MARK } from "../lib/brandMark";
import type { Insights } from "../lib/ipc";
import { band } from "./ActivityHeatmap";

export const CARD_WIDTH = 1200;
export const CARD_HEIGHT = 630;
const WEEKS = 26;

/** Shrimp's brand on a zinc card: white type, and the activity in coral. */
const COLORS = {
  background: "#18181b",
  ink: "#fafafa",
  muted: "#a1a1aa",
  accent: "#f05a38",
  // No words, then coral at 30%, 52%, 76% and 100% over the background.
  heat: ["#27272a", "#592c24", "#883a2a", "#bc4a31", "#f05a38"],
};

const DISPLAY_FONT =
  '-apple-system, BlinkMacSystemFont, "Segoe UI Variable Display", "Segoe UI", system-ui, sans-serif';
const TEXT_FONT =
  '-apple-system, BlinkMacSystemFont, "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif';

const MARGIN = 80;
const COLUMN = (CARD_WIDTH - 2 * MARGIN) / 4;
const CELL = 28;
const GAP = 6;
const HEAT_TOP = 272;

/** How tall Shrimp's mark is in the footer. */
const MARK_SIZE = 28;

const dayKey = (date: Date) =>
  `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;

/** Draws the share card (STA-03): totals, speed, streak, top app and the last half year. */
export function drawShareCard(
  canvas: HTMLCanvasElement,
  insights: Insights,
  t: TFunction,
  language: string,
  today: Date = new Date(),
): boolean {
  const context = canvas.getContext("2d");
  if (!context) {
    return false;
  }
  canvas.width = CARD_WIDTH;
  canvas.height = CARD_HEIGHT;
  const number = new Intl.NumberFormat(language);
  const display = (weight: number, size: number) => `${weight} ${size}px ${DISPLAY_FONT}`;
  const text = (weight: number, size: number) => `${weight} ${size}px ${TEXT_FONT}`;

  context.fillStyle = COLORS.background;
  context.fillRect(0, 0, CARD_WIDTH, CARD_HEIGHT);

  context.fillStyle = COLORS.ink;
  context.font = display(600, 40);
  context.fillText(fit(context, t("share.cardTitle"), CARD_WIDTH - 2 * MARGIN), MARGIN, 104);

  const stats: [string, string][] = [
    [number.format(insights.totalWords), t("share.totalWords")],
    [number.format(insights.wpm), t("share.wpm")],
    [t("insights.days", { count: insights.longestStreak }), t("share.streak")],
    [insights.topApp ?? "—", t("share.topApp")],
  ];
  stats.forEach(([value, label], index) => {
    const x = MARGIN + index * COLUMN;
    context.fillStyle = COLORS.ink;
    context.font = display(600, 56);
    context.fillText(fit(context, value, COLUMN - 24), x, 200);
    context.fillStyle = COLORS.muted;
    context.font = text(500, 22);
    context.fillText(fit(context, label, COLUMN - 24), x, 234);
  });

  // The last half year, Sunday to Saturday in each column.
  const words = new Map(insights.days.map((day) => [day.day, day.words]));
  const start = new Date(today.getFullYear(), today.getMonth(), today.getDate() - today.getDay());
  start.setDate(start.getDate() - (WEEKS - 1) * 7);
  for (let week = 0; week < WEEKS; week++) {
    for (let weekday = 0; weekday < 7; weekday++) {
      const date = new Date(
        start.getFullYear(),
        start.getMonth(),
        start.getDate() + week * 7 + weekday,
      );
      if (date > today) {
        continue;
      }
      context.fillStyle = COLORS.heat[band(words.get(dayKey(date)) ?? 0)] ?? COLORS.heat[0]!;
      context.beginPath();
      context.roundRect(
        MARGIN + week * (CELL + GAP),
        HEAT_TOP + weekday * (CELL + GAP),
        CELL,
        CELL,
        6,
      );
      context.fill();
    }
  }

  // The footer, after Shrimp's mark in coral.
  const footer = CARD_HEIGHT - 74;
  if (typeof Path2D !== "undefined") {
    context.save();
    context.fillStyle = COLORS.accent;
    context.translate(MARGIN, footer - MARK_SIZE + 6);
    context.scale(MARK_SIZE / BRAND_MARK.size, MARK_SIZE / BRAND_MARK.size);
    for (const path of BRAND_MARK.paths) {
      context.fill(new Path2D(path));
    }
    context.restore();
  }
  context.fillStyle = COLORS.muted;
  context.font = text(500, 22);
  context.fillText(t("share.footer"), MARGIN + MARK_SIZE + 12, footer);
  return true;
}

function fit(context: CanvasRenderingContext2D, text: string, width: number): string {
  if (context.measureText(text).width <= width) {
    return text;
  }
  let cut = text;
  while (cut.length > 1 && context.measureText(`${cut}…`).width > width) {
    cut = cut.slice(0, -1);
  }
  return `${cut}…`;
}

/** The canvas as PNG bytes. */
export function pngBytes(canvas: HTMLCanvasElement): Promise<Uint8Array> {
  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => {
      if (!blob) {
        reject(new Error("no image"));
        return;
      }
      blob
        .arrayBuffer()
        .then((buffer) => resolve(new Uint8Array(buffer)))
        .catch(reject);
    }, "image/png");
  });
}
