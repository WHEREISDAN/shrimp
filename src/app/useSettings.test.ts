import { describe, expect, it } from "vitest";
import { SETTINGS } from "./testBackend";
import { newer } from "./useSettings";

describe("newer", () => {
  it("drops a settings change that arrives after a newer one", () => {
    const old = { ...SETTINGS, revision: 4 };
    const fresh = { ...SETTINGS, revision: 5 };
    expect(newer(null, old)).toBe(old);
    expect(newer(fresh, old)).toBe(fresh);
    expect(newer(old, fresh)).toBe(fresh);
  });
});
