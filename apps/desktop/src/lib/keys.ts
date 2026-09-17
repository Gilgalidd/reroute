// Keyboard handling for the picker, kept free of DOM so it can be unit-tested.

export type PickerKey =
  | { kind: "select"; index: number }
  | { kind: "move"; delta: number }
  | { kind: "confirm" }
  | { kind: "menu" }
  | { kind: "cancel" }
  | { kind: "settings" }
  | { kind: "none" };

/**
 * Map a keyboard event to a picker action.
 * Digits 1–9 pick a tile directly, arrows move the highlight, Enter
 * confirms, Space or the context-menu key opens the launch menu, Escape
 * cancels and `,` opens settings (as in many desktop apps).
 */
export function mapKey(key: string, columns: number): PickerKey {
  if (/^[1-9]$/.test(key)) return { kind: "select", index: Number(key) - 1 };
  switch (key) {
    case "ArrowRight":
      return { kind: "move", delta: 1 };
    case "ArrowLeft":
      return { kind: "move", delta: -1 };
    case "ArrowDown":
      return { kind: "move", delta: columns };
    case "ArrowUp":
      return { kind: "move", delta: -columns };
    case "Enter":
      return { kind: "confirm" };
    case " ":
    case "ContextMenu":
      return { kind: "menu" };
    case "Escape":
      return { kind: "cancel" };
    case ",":
      return { kind: "settings" };
    default:
      return { kind: "none" };
  }
}

/** Move a highlight index by `delta`, wrapping around `count` items. */
export function moveIndex(current: number, delta: number, count: number): number {
  if (count <= 0) return 0;
  return (((current + delta) % count) + count) % count;
}
