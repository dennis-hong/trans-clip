import { describe, expect, it } from "vitest";
import { normalizeSourceLanguage } from "./languageArgs";

describe("normalizeSourceLanguage", () => {
  it("returns undefined for auto", () => {
    expect(normalizeSourceLanguage("auto")).toBeUndefined();
  });

  it("returns undefined for missing value", () => {
    expect(normalizeSourceLanguage()).toBeUndefined();
  });

  it("returns concrete language as-is", () => {
    expect(normalizeSourceLanguage("ko")).toBe("ko");
    expect(normalizeSourceLanguage("en")).toBe("en");
  });
});
