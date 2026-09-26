import { describe, expect, it } from "vitest";
import { columnCount, mapKey, moveIndex, shortcutFor } from "./keys";

describe("mapKey", () => {
  it("maps digits to tiles", () => {
    expect(mapKey("1", 4)).toEqual({ kind: "select", index: 0 });
    expect(mapKey("9", 4)).toEqual({ kind: "select", index: 8 });
    expect(mapKey("0", 4)).toEqual({ kind: "none" });
  });

  it("moves by row width for vertical arrows", () => {
    expect(mapKey("ArrowDown", 4)).toEqual({ kind: "move", delta: 4 });
    expect(mapKey("ArrowUp", 3)).toEqual({ kind: "move", delta: -3 });
    expect(mapKey("ArrowLeft", 3)).toEqual({ kind: "move", delta: -1 });
  });

  it("handles confirm, menu, cancel and settings", () => {
    expect(mapKey("Enter", 1)).toEqual({ kind: "confirm" });
    expect(mapKey(" ", 1)).toEqual({ kind: "menu" });
    expect(mapKey("Escape", 1)).toEqual({ kind: "cancel" });
    expect(mapKey(",", 1)).toEqual({ kind: "settings" });
    expect(mapKey("x", 1)).toEqual({ kind: "none" });
  });
});

describe("shortcutFor", () => {
  it("labels exactly the tiles a digit can pick", () => {
    for (let index = 0; index < 9; index++) {
      const digit = shortcutFor(index);
      expect(digit).not.toBeNull();
      expect(mapKey(digit ?? "", 4)).toEqual({ kind: "select", index });
    }
    expect(shortcutFor(9)).toBeNull();
    expect(shortcutFor(12)).toBeNull();
    expect(shortcutFor(-1)).toBeNull();
  });
});

describe("columnCount", () => {
  it("is fixed for the lists and follows the width for tiles", () => {
    expect(columnCount("list", 560)).toBe(1);
    expect(columnCount("two-columns", 560)).toBe(2);
    expect(columnCount("tiles", 560)).toBe(4);
    expect(columnCount("tiles", 0)).toBe(1);
  });
});

describe("moveIndex", () => {
  it("wraps in both directions", () => {
    expect(moveIndex(0, -1, 5)).toBe(4);
    expect(moveIndex(4, 1, 5)).toBe(0);
    expect(moveIndex(2, 3, 5)).toBe(0);
    expect(moveIndex(1, -4, 5)).toBe(2);
  });

  it("is safe with no items", () => {
    expect(moveIndex(3, 1, 0)).toBe(0);
  });
});
