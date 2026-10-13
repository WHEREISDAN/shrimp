import { describe, expect, it } from "vitest";
import { diffWords } from "./diff";

describe("diffWords", () => {
  it("marks removed and added words and keeps the rest", () => {
    expect(diffWords("teh report is late", "The report is very late.")).toEqual([
      { kind: "removed", text: "teh" },
      { kind: "added", text: "The" },
      { kind: "same", text: " report is " },
      { kind: "removed", text: "late" },
      { kind: "added", text: "very late." },
    ]);
  });

  it("handles identical and empty texts", () => {
    expect(diffWords("same", "same")).toEqual([{ kind: "same", text: "same" }]);
    expect(diffWords("", "new")).toEqual([{ kind: "added", text: "new" }]);
  });
});
