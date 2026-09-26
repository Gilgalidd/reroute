import { describe, expect, it } from "vitest";
import { splitLines } from "./lines";

describe("splitLines", () => {
  it("drops blanks and trims", () => {
    expect(splitLines(" a \n\n b\r\n")).toEqual(["a", "b"]);
  });

  it("keeps the spaces inside a line", () => {
    expect(splitLines("--profile-directory=Profile 1\n%URL%")).toEqual(["--profile-directory=Profile 1", "%URL%"]);
  });
});
