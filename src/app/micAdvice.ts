/** The "Find the right mic for you" walkthrough (MIC-10): three questions, one recommendation. */

export type Place = "quiet" | "shared" | "moving";
export type Computer = "laptop" | "desktop";
export type Headphones = "none" | "wired" | "bluetooth";

export interface MicAnswers {
  place: Place;
  computer: Computer;
  headphones: Headphones;
}

export interface MicAdvice {
  title: string;
  why: string;
  tips: string[];
}

export const QUESTIONS = {
  place: {
    label: "Where do you dictate most?",
    options: [
      { value: "quiet", label: "A quiet room" },
      { value: "shared", label: "A shared office or café" },
      { value: "moving", label: "On the move" },
    ],
  },
  computer: {
    label: "What do you dictate on?",
    options: [
      { value: "laptop", label: "A MacBook" },
      { value: "desktop", label: "A desktop Mac (mini, iMac, Studio)" },
    ],
  },
  headphones: {
    label: "Do you wear headphones while you dictate?",
    options: [
      { value: "none", label: "No" },
      { value: "wired", label: "Wired headphones or a headset" },
      { value: "bluetooth", label: "AirPods or other Bluetooth headphones" },
    ],
  },
} as const;

const BLUETOOTH_TIP =
  "Keep Bluetooth headphones for listening: their microphones sound muffled, take time to start and switch your audio to a low-quality mode.";
const WHISPER_TIP = "Speaking quietly is fine: Shrimp raises soft and whispered speech.";

export function recommend({ place, computer, headphones }: MicAnswers): MicAdvice {
  const tips: string[] = [];
  if (headphones === "bluetooth") {
    tips.push(BLUETOOTH_TIP);
  }
  if (place !== "quiet") {
    tips.push(WHISPER_TIP);
  }
  if (place === "moving") {
    return {
      title: computer === "laptop" ? "A wired headset" : "A wired headset or earbuds with a mic",
      why: "A microphone that moves with you stays close to your mouth wherever you are, so background noise stays low.",
      tips: [...tips, "Choose it in the picker, or rank it first so Auto-detect uses it."],
    };
  }
  if (place === "shared") {
    return {
      title:
        headphones === "wired"
          ? "Your wired headset"
          : "A headset or a USB microphone close to you",
      why: "In a noisy room, a microphone near your mouth hears you instead of the room.",
      tips: [...tips, "Rank it first so Auto-detect picks it whenever it is plugged in."],
    };
  }
  if (computer === "desktop") {
    return {
      title: "A USB microphone on your desk",
      why: "Desktop Macs other than the iMac have no microphone of their own, and a USB microphone gives clear, low-noise audio.",
      tips: [...tips, "Place it 15–30 cm from your mouth, a little to the side."],
    };
  }
  return {
    title: "Your MacBook's built-in microphone",
    why: "In a quiet room it is clear, always there and starts instantly.",
    tips: [...tips, "With the lid closed it can't hear you, so connect another microphone then."],
  };
}
