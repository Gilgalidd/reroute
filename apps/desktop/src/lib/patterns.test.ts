import { describe, expect, it } from "vitest";
import { parsePattern, patternProblem } from "./patterns";

describe("parsePattern", () => {
  it("recognises the three kinds", () => {
    expect(parsePattern("domain:*.github.com")).toEqual({ kind: "domain", body: "*.github.com" });
    expect(parsePattern("regex: ^https")).toEqual({ kind: "regex", body: "^https" });
    expect(parsePattern("exact:https://a.org/")).toEqual({ kind: "exact", body: "https://a.org/" });
  });

  it("treats bare URLs and unknown prefixes as exact", () => {
    expect(parsePattern("https://a.org/")).toEqual({ kind: "exact", body: "https://a.org/" });
    expect(parsePattern("glob:*")).toEqual({ kind: "exact", body: "glob:*" });
  });
});

describe("patternProblem", () => {
  it("flags empty, bad hosts and bad regexes", () => {
    expect(patternProblem("domain:")).toBe("empty pattern");
    expect(patternProblem("domain:a b")).toBe("not a valid host name");
    expect(patternProblem("domain:*.github.com")).toBeNull();
    expect(patternProblem("regex:(")).toBe("not a valid regular expression");
    expect(patternProblem("regex:^https://")).toBeNull();
    expect(patternProblem("https://a.org")).toBeNull();
  });
});
