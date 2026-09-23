import { describe, expect, it } from "vitest";
import { splitName } from "./names";

describe("splitName", () => {
  it("separates the mode a private entry was named with", () => {
    expect(splitName("Firefox (Private)")).toEqual({ base: "Firefox", mode: "Private" });
    expect(splitName("Chromium Web Browser (Incognito)")).toEqual({
      base: "Chromium Web Browser",
      mode: "Incognito",
    });
    expect(splitName("Microsoft Edge (InPrivate)")).toEqual({
      base: "Microsoft Edge",
      mode: "InPrivate",
    });
  });

  it("leaves an ordinary name alone", () => {
    expect(splitName("Firefox")).toEqual({ base: "Firefox", mode: null });
    expect(splitName("  Firefox  ")).toEqual({ base: "Firefox", mode: null });
    expect(splitName("Firefox (Developer Edition) nightly")).toEqual({
      base: "Firefox (Developer Edition) nightly",
      mode: null,
    });
    expect(splitName("()")).toEqual({ base: "()", mode: null });
  });
});
