/** The "Find the right mic for you" walkthrough (MIC-10): three questions, one recommendation. */
import type { controls } from "../i18n/locales/en/controls";

export type Place = "quiet" | "shared" | "moving";
export type Computer = "laptop" | "desktop";
export type Headphones = "none" | "wired" | "bluetooth";

export interface MicAnswers {
  place: Place;
  computer: Computer;
  headphones: Headphones;
}

type Text = typeof controls.micAdvice;

/** A recommendation as interface text keys, translated where it is shown. */
export interface MicAdvice {
  title: `micAdvice.titles.${keyof Text["titles"]}`;
  why: `micAdvice.why.${keyof Text["why"]}`;
  tips: `micAdvice.tips.${keyof Text["tips"]}`[];
}

/** The questions and their answers, with the keys of their text. */
export const QUESTIONS = {
  place: {
    label: "micAdvice.questions.place.label",
    options: [
      { value: "quiet", label: "micAdvice.questions.place.quiet" },
      { value: "shared", label: "micAdvice.questions.place.shared" },
      { value: "moving", label: "micAdvice.questions.place.moving" },
    ],
  },
  computer: {
    label: "micAdvice.questions.computer.label",
    options: [
      { value: "laptop", label: "micAdvice.questions.computer.laptop" },
      { value: "desktop", label: "micAdvice.questions.computer.desktop" },
    ],
  },
  headphones: {
    label: "micAdvice.questions.headphones.label",
    options: [
      { value: "none", label: "micAdvice.questions.headphones.none" },
      { value: "wired", label: "micAdvice.questions.headphones.wired" },
      { value: "bluetooth", label: "micAdvice.questions.headphones.bluetooth" },
    ],
  },
} as const;

export function recommend({ place, computer, headphones }: MicAnswers): MicAdvice {
  const tips: MicAdvice["tips"] = [];
  if (headphones === "bluetooth") {
    tips.push("micAdvice.tips.bluetooth");
  }
  if (place !== "quiet") {
    tips.push("micAdvice.tips.whisper");
  }
  if (place === "moving") {
    return {
      title:
        computer === "laptop"
          ? "micAdvice.titles.wiredHeadset"
          : "micAdvice.titles.wiredHeadsetOrEarbuds",
      why: "micAdvice.why.moving",
      tips: [...tips, "micAdvice.tips.moving"],
    };
  }
  if (place === "shared") {
    return {
      title:
        headphones === "wired"
          ? "micAdvice.titles.yourWiredHeadset"
          : "micAdvice.titles.headsetOrUsb",
      why: "micAdvice.why.shared",
      tips: [...tips, "micAdvice.tips.shared"],
    };
  }
  if (computer === "desktop") {
    return {
      title: "micAdvice.titles.usbMicrophone",
      why: "micAdvice.why.desktop",
      tips: [...tips, "micAdvice.tips.desktop"],
    };
  }
  return {
    title: "micAdvice.titles.builtIn",
    why: "micAdvice.why.quiet",
    tips: [...tips, "micAdvice.tips.quiet"],
  };
}
