import { describe, expect, it } from "vitest";
import { columnCount, isMac, mapKey, moveIndex, shortcutFor } from "./keys";

describe("mapKey", () => {
  const key = (k: string, columns = 4) => mapKey({ key: k }, columns, false);

  it("maps digits to tiles", () => {
    expect(key("1")).toEqual({ kind: "select", index: 0, private: false });
    expect(key("9")).toEqual({ kind: "select", index: 8, private: false });
    expect(key("0")).toEqual({ kind: "none" });
  });

  it("reads a digit from the key's place, whatever the layout types", () => {
    // AZERTY: the top row types & without Shift and 1 with it.
    expect(mapKey({ key: "&", code: "Digit1" }, 4, false)).toEqual({ kind: "select", index: 0, private: false });
    expect(mapKey({ key: "1", code: "Digit1", shiftKey: true }, 4, false)).toEqual({ kind: "select", index: 0, private: true });
    expect(mapKey({ key: "3", code: "Numpad3" }, 4, false)).toEqual({ kind: "select", index: 2, private: false });
    expect(mapKey({ key: "0", code: "Digit0" }, 4, false)).toEqual({ kind: "none" });
  });

  it("opens the private entry with Shift", () => {
    expect(mapKey({ key: "!", code: "Digit1", shiftKey: true }, 4, false)).toEqual({ kind: "select", index: 0, private: true });
    expect(mapKey({ key: "Enter", shiftKey: true }, 4, false)).toEqual({ kind: "confirm", private: true });
  });

  it("moves by row width for vertical arrows", () => {
    expect(key("ArrowDown", 4)).toEqual({ kind: "move", delta: 4 });
    expect(key("ArrowUp", 3)).toEqual({ kind: "move", delta: -3 });
    expect(key("ArrowLeft", 3)).toEqual({ kind: "move", delta: -1 });
  });

  it("handles confirm, menu, cancel and settings", () => {
    expect(key("Enter", 1)).toEqual({ kind: "confirm", private: false });
    expect(key(" ", 1)).toEqual({ kind: "menu" });
    expect(key("Escape", 1)).toEqual({ kind: "cancel" });
    expect(key(",", 1)).toEqual({ kind: "settings" });
    expect(key("x", 1)).toEqual({ kind: "none" });
  });

  it("copies and edits the link with the system's command key", () => {
    expect(mapKey({ key: "c", ctrlKey: true }, 4, false)).toEqual({ kind: "copy" });
    expect(mapKey({ key: "L", ctrlKey: true, shiftKey: true }, 4, false)).toEqual({ kind: "edit" });
    expect(mapKey({ key: "c", metaKey: true }, 4, true)).toEqual({ kind: "copy" });
    expect(mapKey({ key: "l", metaKey: true }, 4, true)).toEqual({ kind: "edit" });
    // Ctrl is not the command key on macOS, nor ⌘ elsewhere.
    expect(mapKey({ key: "c", ctrlKey: true }, 4, true)).toEqual({ kind: "none" });
    expect(mapKey({ key: "c", metaKey: true }, 4, false)).toEqual({ kind: "none" });
  });

  it("leaves other combinations to the system", () => {
    expect(mapKey({ key: "1", code: "Digit1", ctrlKey: true }, 4, false)).toEqual({ kind: "none" });
    expect(mapKey({ key: "Enter", altKey: true }, 4, false)).toEqual({ kind: "none" });
    expect(mapKey({ key: "c", ctrlKey: true, altKey: true }, 4, false)).toEqual({ kind: "none" });
    expect(mapKey({ key: "v", ctrlKey: true }, 4, false)).toEqual({ kind: "none" });
  });
});

describe("isMac", () => {
  it("knows Apple's platforms", () => {
    expect(isMac("MacIntel")).toBe(true);
    expect(isMac("iPad")).toBe(true);
    expect(isMac("Win32")).toBe(false);
    expect(isMac("Linux x86_64")).toBe(false);
    expect(isMac("")).toBe(false);
  });
});

describe("shortcutFor", () => {
  it("labels exactly the tiles a digit can pick", () => {
    for (let index = 0; index < 9; index++) {
      const digit = shortcutFor(index);
      expect(digit).not.toBeNull();
      expect(mapKey({ key: digit ?? "" }, 4, false)).toEqual({ kind: "select", index, private: false });
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
