import { describe, expect, it } from "vitest";
import { recommend } from "./micAdvice";

describe("recommend", () => {
  it("recommends the built-in microphone for a MacBook in a quiet room", () => {
    const advice = recommend({ place: "quiet", computer: "laptop", headphones: "none" });
    expect(advice.title).toBe("Your MacBook's built-in microphone");
  });

  it("recommends a USB microphone for a desktop Mac", () => {
    const advice = recommend({ place: "quiet", computer: "desktop", headphones: "none" });
    expect(advice.title).toBe("A USB microphone on your desk");
  });

  it("warns about Bluetooth microphones and suggests a close microphone in noise", () => {
    const advice = recommend({ place: "shared", computer: "laptop", headphones: "bluetooth" });
    expect(advice.title).toBe("A headset or a USB microphone close to you");
    expect(advice.tips[0]).toMatch(/Bluetooth headphones for listening/);
    expect(advice.tips).toContain(
      "Speaking quietly is fine: Shrimp raises soft and whispered speech.",
    );
  });

  it("recommends a wired headset on the move", () => {
    expect(recommend({ place: "moving", computer: "laptop", headphones: "wired" }).title).toBe(
      "A wired headset",
    );
  });
});
