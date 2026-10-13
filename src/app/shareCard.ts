import type { TFunction } from "i18next";
import type { Insights } from "../lib/ipc";
import { band } from "./ActivityHeatmap";

export const CARD_WIDTH = 1200;
export const CARD_HEIGHT = 630;
const WEEKS = 26;

const COLORS = {
  background: "#fbfaf8",
  ink: "#1d1d1f",
  muted: "#6e6e73",
  accent: "#d6471f",
  heat: ["#ebe7e2", "#f6c7b5", "#ee9a7a", "#e2703f", "#d6471f"],
};

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
  const font = (weight: number, size: number) =>
    `${weight} ${size}px -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif`;

  context.fillStyle = COLORS.background;
  context.fillRect(0, 0, CARD_WIDTH, CARD_HEIGHT);
  context.fillStyle = COLORS.accent;
  context.fillRect(0, 0, CARD_WIDTH, 12);

  context.fillStyle = COLORS.ink;
  context.font = font(700, 54);
  context.fillText(t("share.cardTitle"), 72, 120);

  const stats: [string, string][] = [
    [number.format(insights.totalWords), t("share.totalWords")],
    [number.format(insights.wpm), t("share.wpm")],
    [t("insights.days", { count: insights.longestStreak }), t("share.streak")],
    [insights.topApp ?? "—", t("share.topApp")],
  ];
  stats.forEach(([value, label], index) => {
    const x = 72 + index * 270;
    context.fillStyle = COLORS.ink;
    context.font = font(700, 44);
    context.fillText(fit(context, value, 250), x, 230);
    context.fillStyle = COLORS.muted;
    context.font = font(500, 24);
    context.fillText(fit(context, label, 250), x, 268);
  });

  // The last half year, Sunday to Saturday in each column.
  const words = new Map(insights.days.map((day) => [day.day, day.words]));
  const cell = 26;
  const gap = 6;
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
      context.fillRect(72 + week * (cell + gap), 320 + weekday * (cell + gap), cell, cell);
    }
  }

  context.fillStyle = COLORS.muted;
  context.font = font(500, 24);
  context.fillText(t("share.footer"), 72, CARD_HEIGHT - 40);
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
