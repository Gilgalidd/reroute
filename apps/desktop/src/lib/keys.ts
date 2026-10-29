// Keyboard handling for the picker, kept free of DOM so it can be unit-tested.

import type { PickerLayout } from "./types";

/** What the picker reads of a key press: a `KeyboardEvent` has all of it. */
export interface KeyPress {
  key: string;
  /** The key's place on the keyboard, which names a digit whatever the layout. */
  code?: string;
  shiftKey?: boolean;
  ctrlKey?: boolean;
  metaKey?: boolean;
  altKey?: boolean;
}

export type PickerKey =
  | { kind: "select"; index: number; private: boolean }
  | { kind: "move"; delta: number }
  | { kind: "confirm"; private: boolean }
  | { kind: "menu" }
  | { kind: "copy" }
  | { kind: "edit" }
  | { kind: "cancel" }
  | { kind: "settings" }
  | { kind: "none" };

const DIGIT_CODE = /^(?:Digit|Numpad)([1-9])$/;

/**
 * Map a key press to a picker action.
 * Digits 1–9 pick a tile directly, arrows move the highlight, Enter
 * confirms, and with Shift a digit or Enter opens the browser's private
 * entry instead. Space or the context-menu key opens the launch menu,
 * Escape cancels and `,` opens settings (as in many desktop apps). The
 * system's command key (`⌘` on macOS, `Ctrl` elsewhere) with C copies the
 * link, with L edits it, as in a browser's address bar; any other
 * combination with it, or with Alt, is left to the system.
 *
 * A digit is read from the key's place, so the top row picks a tile on
 * layouts where it types other characters without Shift, such as AZERTY.
 */
export function mapKey(press: KeyPress, columns: number, mac: boolean): PickerKey {
  const command = mac ? press.metaKey : press.ctrlKey;
  if (command && !press.altKey) {
    switch (press.key.toLowerCase()) {
      case "c":
        return { kind: "copy" };
      case "l":
        return { kind: "edit" };
      default:
        return { kind: "none" };
    }
  }
  if (press.ctrlKey || press.metaKey || press.altKey) return { kind: "none" };
  const privately = press.shiftKey === true;
  const digit = DIGIT_CODE.exec(press.code ?? "")?.[1] ?? (/^[1-9]$/.test(press.key) ? press.key : null);
  if (digit) return { kind: "select", index: Number(digit) - 1, private: privately };
  switch (press.key) {
    case "ArrowRight":
      return { kind: "move", delta: 1 };
    case "ArrowLeft":
      return { kind: "move", delta: -1 };
    case "ArrowDown":
      return { kind: "move", delta: columns };
    case "ArrowUp":
      return { kind: "move", delta: -columns };
    case "Enter":
      return { kind: "confirm", private: privately };
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

/** Is `platform` (`navigator.platform`) Apple's, whose command key is `⌘`? */
export function isMac(platform: string): boolean {
  return /^(Mac|iPhone|iPad)/.test(platform);
}

/**
 * The digit that picks the tile at `index`, or `null` from the tenth tile
 * on, which has no key. Tiles show this as their badge, so a number is
 * only ever displayed when pressing it does something.
 */
export function shortcutFor(index: number): string | null {
  return index >= 0 && index < 9 ? String(index + 1) : null;
}

/**
 * Browsers per row of the picker, which is how far the up and down arrows
 * move. The lists have the number of columns their name says; tiles fill
 * the row, each taking 116px of the grid's `width` (a 104px minimum plus
 * the gap, see `.grid` in Picker.svelte).
 */
export function columnCount(layout: PickerLayout, width: number): number {
  switch (layout) {
    case "list":
      return 1;
    case "two-columns":
      return 2;
    case "tiles":
      return Math.max(1, Math.floor(width / 116));
  }
}

/** Move a highlight index by `delta`, wrapping around `count` items. */
export function moveIndex(current: number, delta: number, count: number): number {
  if (count <= 0) return 0;
  return (((current + delta) % count) + count) % count;
}
