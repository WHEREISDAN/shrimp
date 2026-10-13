import { describe, expect, it } from "vitest";
import { insertLink, plainPreview, wrapSelection } from "./markdownEdit";

describe("plainPreview", () => {
  it("strips emphasis, escapes and link targets onto one line", () => {
    expect(plainPreview("**Hi** _there_, *you*\n\n- see [the docs](https://x.dev/a_b)")).toBe(
      "Hi there, you - see the docs",
    );
  });

  it("keeps escaped marks as literal characters", () => {
    expect(plainPreview("2 \\* 3 \\_ 4")).toBe("2 * 3 _ 4");
  });

  it("strips marks inside link text", () => {
    expect(plainPreview("[**bold** link](https://x.dev)")).toBe("bold link");
  });
});

describe("wrapSelection", () => {
  it("wraps the selection and keeps it selected", () => {
    expect(wrapSelection("say hello now", 4, 9, "**")).toEqual({
      value: "say **hello** now",
      start: 6,
      end: 11,
    });
  });

  it("inserts an empty pair with the caret inside", () => {
    expect(wrapSelection("ab", 1, 1, "*")).toEqual({ value: "a**b", start: 2, end: 2 });
  });
});

describe("insertLink", () => {
  it("links the selection and selects the address", () => {
    const edit = insertLink("see docs", 4, 8);
    expect(edit.value).toBe("see [docs](https://)");
    expect(edit.value.slice(edit.start, edit.end)).toBe("https://");
  });

  it("uses placeholder text without a selection", () => {
    const edit = insertLink("", 0, 0);
    expect(edit.value).toBe("[link text](https://)");
    expect(edit.value.slice(edit.start, edit.end)).toBe("https://");
  });
});
