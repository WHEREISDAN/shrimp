import { describe, expect, it } from "vitest";
import { i18n } from "../i18n";
import { recommend, type MicAnswers } from "./micAdvice";

/** The recommendation in English, as the dialog shows it. */
function advise(answers: MicAnswers) {
  const advice = recommend(answers);
  return { title: i18n.t(advice.title), tips: advice.tips.map((tip) => i18n.t(tip)) };
}

describe("recommend", () => {
  it("recommends the built-in microphone for a MacBook in a quiet room", () => {
    const advice = advise({ place: "quiet", computer: "laptop", headphones: "none" });
    expect(advice.title).toBe("Your MacBook's built-in microphone");
  });

  it("recommends a USB microphone for a desktop Mac", () => {
    const advice = advise({ place: "quiet", computer: "desktop", headphones: "none" });
    expect(advice.title).toBe("A USB microphone on your desk");
  });

  it("warns about Bluetooth microphones and suggests a close microphone in noise", () => {
    const advice = advise({ place: "shared", computer: "laptop", headphones: "bluetooth" });
    expect(advice.title).toBe("A headset or a USB microphone close to you");
    expect(advice.tips[0]).toMatch(/Bluetooth headphones for listening/);
    expect(advice.tips).toContain(
      "Speaking quietly is fine: Shrimp raises soft and whispered speech.",
    );
  });

  it("recommends a wired headset on the move", () => {
    expect(advise({ place: "moving", computer: "laptop", headphones: "wired" }).title).toBe(
      "A wired headset",
    );
  });
});
